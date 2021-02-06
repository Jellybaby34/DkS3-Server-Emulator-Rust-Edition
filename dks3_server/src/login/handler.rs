use std::default::Default;

use std::net::SocketAddr;
use std::sync::Arc;

use bytes::BytesMut;
use parking_lot::RwLock;
use prost::Message;
use tokio::io;
use tokio::net::TcpStream;

use tracing::{debug, error, info, span, trace, warn, Level};

use dks3_proto::frame::Frame;
use dks3_proto::msg::frpg2_request;
use dks3_proto::msg::frpg2_request::RequestQueryLoginServerInfoResponse;
use frpg2_request::RequestQueryLoginServerInfo;

use crate::connection::Connection;
use crate::server::RsaManager;
use crate::Config;

pub struct LoginConnectionHandler {
    connection: Connection,
    global_counter: u16,
    counter: u32,
    config: Config,
    rsa_manager: RsaManager,
}

impl LoginConnectionHandler {
    pub async fn write_message<M: Message>(&mut self, message: M) {
        let message_len = message.encoded_len();
        let mut message_data = BytesMut::with_capacity(message_len);

        // TODO: handle error
        let _ = message.encode(&mut message_data);

        let mut message_encrypted = BytesMut::new();
        message_encrypted.reserve(1024);

        let message_encrypted_len = self
            .rsa_manager
            .rsa_encrypt(&message_data, &mut message_encrypted);

        message_encrypted.truncate(message_encrypted_len);

        self.connection
            .write_frame(Frame::new(
                self.global_counter,
                self.counter,
                message_encrypted,
            ))
            .await;
    }

    pub async fn read_message<M: Message + Default>(&mut self) -> M {
        let frame = self.connection.read_frame().await.unwrap();

        let mut payload_decrypted = BytesMut::new();
        payload_decrypted.resize(256, 0);

        let payload_length_decrypted = self
            .rsa_manager
            .rsa_decrypt(&frame.data, &mut payload_decrypted);

        payload_decrypted.truncate(payload_length_decrypted);
        let msg = M::decode(payload_decrypted).unwrap(); // TODO: handle error

        self.global_counter = frame.global_counter;
        self.counter = frame.counter;

        msg
    }

    pub fn new(
        stream: TcpStream,
        config: Config,
        rsa_manager: RsaManager,
    ) -> LoginConnectionHandler {
        let (stream_reader, stream_writer) = io::split(stream);
        let connection = Connection::start(stream_reader, stream_writer);

        LoginConnectionHandler {
            connection,
            global_counter: 0,
            counter: 0,
            config,
            rsa_manager,
        }
    }

    pub async fn run(&mut self) {
        let server_info_req = self.read_message::<RequestQueryLoginServerInfo>().await;

        /* Could check steam ID, versionnum, etc. here */
        info!(
            "Client connected with steamid {}, version {}",
            server_info_req.steamid, server_info_req.versionnum
        );

        let mut server_info: RequestQueryLoginServerInfoResponse = Default::default();
        server_info.serverip = self.config.get_server_ip().to_string();
        server_info.port = self.config.get_auth_port() as i64;

        self.write_message(server_info).await;
    }
}

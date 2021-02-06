use std::default::Default;

use bytes::BytesMut;
use prost::Message;
use tokio::io;
use tokio::net::TcpStream;
use tracing::{debug, error, info, span, trace, warn, Level};

use dks3_proto::frame::Frame;
use dks3_proto::msg::frpg2_request;
use dks3_proto::msg::frpg2_request::RequestQueryLoginServerInfoResponse;
use frpg2_request::RequestQueryLoginServerInfo;

use crate::net::{CipherMode, Connection};
use crate::server::RsaManager;
use crate::Config;
use std::time::Duration;

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

        self.connection
            .write_frame(Frame::new(self.global_counter, self.counter, message_data))
            .await;
    }

    pub async fn read_message<M: Message + Default>(&mut self) -> M {
        let frame = self.connection.read_frame().await.unwrap();
        let msg = M::decode(frame.data).unwrap(); // TODO: handle error

        self.global_counter = frame.global_counter;
        self.counter = frame.counter;

        msg
    }

    pub async fn new(stream: TcpStream, config: Config) -> LoginConnectionHandler {
        let inbound_cipher_mode = CipherMode::rsa_pkcs1_oeap(config.rsa_private_key.as_bytes());
        let outbound_cipher_mode = CipherMode::rsa_x931(config.rsa_private_key.as_bytes());

        let ciphers = (inbound_cipher_mode, outbound_cipher_mode);
        let connection = Connection::start(ciphers, stream);

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
        info!(steamid = %server_info_req.steamid, version = %server_info_req.versionnum, "Client connected");

        let server_info = RequestQueryLoginServerInfoResponse {
            serverip: self.config.get_server_ip().to_string(),
            port: self.config.get_auth_port() as i64,
        };

        self.write_message(server_info).await;

        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

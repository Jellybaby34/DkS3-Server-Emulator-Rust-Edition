use std::collections::{HashMap, HashSet};
use std::default::Default;
use std::fmt::Write;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::{Buf, Bytes, BytesMut};
use futures::StreamExt;
use parking_lot::{Mutex, RwLock};
use prost::Message;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_util::codec::FramedRead;
use tracing::{debug, error, info, span, trace, warn, Level};

use dks3_proto::frame::FrameDecoder;
use dks3_proto::msg::frpg2_request;
use dks3_proto::msg::frpg2_request::RequestQueryLoginServerInfoResponse;
use frpg2_request::RequestQueryLoginServerInfo;

use crate::connection::Connection;
use crate::server::RsaManager;
use crate::Config;

// Packet header sizes in bytes
pub const CLIENT_TO_SERVER_HEADER: u8 = 26;
pub const SERVER_TO_CLIENT_HEADER: u8 = 0;

pub struct LoginClientInfo {
    pub peer_addr: SocketAddr,
    pub steam_id_string: String,
    pub client_game_version: u64,
    pub packet_counter: u32,
}

pub struct LoginClient {
    connection: Connection,
    config: Arc<RwLock<Config>>,
    rsa_manager: Arc<RwLock<RsaManager>>,
    client_info: LoginClientInfo,
}

pub struct LoginClientSignalingInfo {
    pub channel: mpsc::Sender<Vec<u8>>,
    pub addr_p2p: [u8; 4],
    pub port_p2p: u16,
}

impl LoginClientSignalingInfo {
    pub fn new(channel: mpsc::Sender<Vec<u8>>) -> LoginClientSignalingInfo {
        LoginClientSignalingInfo {
            channel,
            addr_p2p: [0; 4],
            port_p2p: 0,
        }
    }
}

impl LoginClient {
    pub async fn write_message<M: Message>(&mut self, message: M) {}

    pub async fn read_message<M: Message + Default>(&mut self) -> M {
        let frame = self.connection.read_frame().await.unwrap();

        let mut payload_decrypted = BytesMut::new();
        payload_decrypted.resize(256, 0);

        let payload_length_decrypted = self
            .rsa_manager
            .read()
            .rsa_decrypt(&frame.data, &mut payload_decrypted);

        payload_decrypted.truncate(payload_length_decrypted);
        let msg = M::decode(payload_decrypted).unwrap(); // TODO: handle error

        msg
    }

    pub async fn new(
        peer_addr: SocketAddr,
        stream: TcpStream,
        config: Arc<RwLock<Config>>,
        rsa_manager: Arc<RwLock<RsaManager>>,
    ) -> LoginClient {
        let client_info = LoginClientInfo {
            peer_addr,
            steam_id_string: String::new(),
            client_game_version: 0,
            packet_counter: 0,
        };

        let (stream_reader, stream_writer) = io::split(stream);
        let connection = Connection::start(stream_reader, stream_writer);

        LoginClient {
            connection,
            config,
            rsa_manager,
            client_info,
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
        server_info.serverip = self.config.read().get_server_ip().to_string();
        server_info.port = self.config.read().get_auth_port() as i64;
    }
}

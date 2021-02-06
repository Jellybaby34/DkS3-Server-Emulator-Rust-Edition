use std::collections::{HashMap, HashSet};
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
use tracing::{debug, error, info, Level, span, trace, warn};

use dks3_proto::frame::FrameDecoder;
use dks3_proto::msg::frpg2_request;

use crate::Config;
use crate::server::RsaManager;

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
    frame_reader: tokio_util::codec::FramedRead<io::ReadHalf<TcpStream>, FrameDecoder>,
    config: Arc<RwLock<Config>>,
    rsa_manager: Arc<RwLock<RsaManager>>,
    channel_sender: mpsc::Sender<Vec<u8>>,
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
    pub async fn new(
        peer_addr: SocketAddr,
        stream: TcpStream,
        config: Arc<RwLock<Config>>,
        rsa_manager: Arc<RwLock<RsaManager>>,
    ) -> LoginClient {
        let client_info = LoginClientInfo {
            peer_addr: peer_addr,
            steam_id_string: String::new(),
            client_game_version: 0,
            packet_counter: 0,
        };

        let (channel_sender, mut channel_receiver) = mpsc::channel::<Vec<u8>>(32);
        let (stream_reader, mut stream_writer) = io::split(stream);

        let frame_reader = FramedRead::new(stream_reader, FrameDecoder::new(false));

        let fut_sock_writer = async move {
            while let Some(outgoing_packet) = channel_receiver.recv().await {
                let _ = stream_writer.write_all(&outgoing_packet).await;
            }
            let _ = stream_writer.shutdown().await;
        };

        tokio::spawn(fut_sock_writer);

        LoginClient {
            frame_reader,
            config,
            rsa_manager,
            channel_sender,
            client_info,
        }
    }

    ///// Command processing
    pub async fn process(&mut self) {
        loop {
            let r = self.frame_reader.next().await;

            match r {
                Some(Ok(frame)) => {
                    let mut payload_decrypted = BytesMut::new();
                    payload_decrypted.resize(256, 0);

                    let payload_length_decrypted = self.rsa_manager.read().rsa_decrypt(&frame.data, &mut payload_decrypted);

                    let mut s = String::new();
                    for i in 0..payload_length_decrypted {
                        write!(&mut s, "{:02X} ", payload_decrypted[i]).expect("Unable to write")
                    }
                    println!("{}", s);

                    payload_decrypted.truncate(payload_length_decrypted);
                    let test = frpg2_request::RequestQueryLoginServerInfo::decode(payload_decrypted).unwrap();
                    println!("{:?}", test);

                    let _ = self.send_auth_server_info().await;
                }
                Some(Err(e)) => {
                    error!("Error while decoding frame: {}", e);
                    break;
                }
                None => {
                    info!("Client disconnected");
                    break;
                }
            }
        }
    }

    async fn craft_header(&mut self) {}

    async fn send_auth_server_info(&mut self) {
        let mut server_info = frpg2_request::RequestQueryLoginServerInfoResponse::default();
        server_info.serverip = self.config.read().get_server_ip().to_string();
        server_info.port = self.config.read().get_auth_port() as i64;
        println!("Server IP: {}", server_info.serverip);
        let mut payload = BytesMut::with_capacity(server_info.encoded_len());
        server_info.encode(&mut payload).unwrap();
        println!("Len: {}", payload.len());
    }
}


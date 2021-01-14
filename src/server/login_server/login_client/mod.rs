use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::net::SocketAddr;
use std::fmt::Write;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use parking_lot::{Mutex, RwLock};
use bytes::{Bytes, BytesMut, Buf};
use prost::Message;

use tracing::{debug, error, info, span, trace, warn, Level};

use crate::Config;
use crate::server::RsaManager;

// Packet header sizes in bytes
pub const CLIENT_TO_SERVER_HEADER: u8 = 26;
pub const SERVER_TO_CLIENT_HEADER: u8 = 0;

pub mod frpg2_request_message {
    include!(concat!(env!("OUT_DIR"), "/frpg2_request_message.rs"));
}

pub struct LoginClientInfo {
    pub peer_addr: SocketAddr,
    pub steam_id_string: String,
    pub client_game_version: u64,
    pub packet_counter: u32
}

pub struct LoginClient {
    stream_reader: io::ReadHalf<TcpStream>,
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
        rsa_manager: Arc<RwLock<RsaManager>>
    ) -> LoginClient {
        let client_info = LoginClientInfo {
            peer_addr: peer_addr,
            steam_id_string: String::new(),
            client_game_version: 0,
            packet_counter: 0
        };

        let (channel_sender, mut channel_receiver) = mpsc::channel::<Vec<u8>>(32);
        let (stream_reader, mut stream_writer) = io::split(stream);

        let fut_sock_writer = async move {
            while let Some(outgoing_packet) = channel_receiver.recv().await {
                let _ = stream_writer.write_all(&outgoing_packet).await;
            }
            let _ = stream_writer.shutdown().await;
        };

        tokio::spawn(fut_sock_writer);

        LoginClient {
            stream_reader,
            config,
            rsa_manager,
            channel_sender,
            client_info
        }
    }

    ///// Command processing
    pub async fn process(&mut self) {
        loop {
            let mut header_data = BytesMut::with_capacity(CLIENT_TO_SERVER_HEADER as usize);
            let r = self.stream_reader.read_buf(&mut header_data).await;

            match r {
                Ok(header_length) => {
//                    let mut payload_data: [u8; 2000] = [0; 2000];
                    let mut payload_data = BytesMut::with_capacity(2048);
                    let payload_length = self.stream_reader.read_buf(&mut payload_data).await.unwrap(); 

                    let mut s = String::new();
                    for i in 0..header_length {
                        write!(&mut s, "{:02X} ", header_data[i]).expect("Unable to write")
                    }
                    println!("{}", s);

                    if self.process_header(header_data, header_length, payload_length).await.is_err() {
                        info!("Disconnecting");
                        break;
                    }

                    // The rust OpenSSL bindings don't let you pass buffer length
                    // so we need to slice the buffer so that it's the expected size
                    let payload_data_slice = &payload_data[0..payload_length];
//                    let mut payload_decrypted: [u8; 256] = [0; 256];
                    let mut payload_decrypted = BytesMut::new();
                    payload_decrypted.resize(256, 0);

                    let payload_length_decrypted = self.rsa_manager.read().rsa_decrypt( &payload_data_slice, &mut payload_decrypted);

                    let mut s = String::new();
                    for i in 0..payload_length_decrypted {
                        write!(&mut s, "{:02X} ", payload_decrypted[i]).expect("Unable to write")
                    }
                    println!("{}", s);
                    
                    payload_decrypted.truncate(payload_length_decrypted);
                    let test = frpg2_request_message::RequestQueryLoginServerInfo::decode(payload_decrypted).unwrap();
                    println!("{:?}", test);

                    let _ = self.send_auth_server_info().await;

                    break;

                }
                Err(e) => {
                    info!("Client disconnected: {}", e);
                    break;
                }
            }
        }
    }

    async fn process_header(&mut self, mut header_data: BytesMut, header_length: usize, payload_length: usize) -> Result<(), ()> {
        if header_length < CLIENT_TO_SERVER_HEADER.into() {
            error!("Received header was shorter than expected");
            return Err(());
        }

        let total_packet_length: u32 = (header_length+payload_length) as u32;
        // Total packet length - 2 in big endian
        let packet_length_1_be = header_data.get_u16();
//        let packet_length_1_be: u16 = u16::from_be_bytes([header_data[0], header_data[1]]);
        if packet_length_1_be != (total_packet_length-2) as u16 {
            error!("Packet length #1 was incorrect, expected: {}, received: {}", (total_packet_length-2), packet_length_1_be);
            return Err(());
        }

        // Counter of total number of packets sent since clients game started, ?big endian
        let _sent_packets_counter = header_data.get_u16(); // This is done to advance the buffer index but we do nothing with the result
//        let sent_packets_counter: u16 = u16::from_be_bytes([header_data[2], header_data[3]]); 
/*
        if sent_packets_counter != (/* */).into() {
            self.log("Sent counter was incorrect");
            return Err(());
        }
*/
        // Always 0x00 0x00
        let unknown_1 = header_data.get_u16();
//        let unknown_1: u16 = u16::from_be_bytes([header_data[4], header_data[5]]); 
        if unknown_1 != 0 {
            error!("Unknown #1 was incorrect, expected: {}, received: {}", 0, unknown_1);
            return Err(());
        }

        // Total packet length - 14 in big endian
        let packet_length_2a_be = header_data.get_u32();
//        let packet_length_2a_be: u32 = u32::from_be_bytes([header_data[6], header_data[7], header_data[8], header_data[9]]); 
        if packet_length_2a_be != (total_packet_length-14) {
            error!("Packet length #2A was incorrect, expected: {}, received: {}", (total_packet_length-14), packet_length_2a_be);
            return Err(());
        }

        // Total packet length - 14 in big endian
        let packet_length_2b_be = header_data.get_u32();
//        let packet_length_2b_be: u32 = u32::from_be_bytes([header_data[10], header_data[11], header_data[12], header_data[13]]); 
        if packet_length_2b_be != (total_packet_length-14) {
            error!("Packet length #2B was incorrect, expected: {}, received: {}", (total_packet_length-14), packet_length_2b_be);
            return Err(());
        }

        // Always 0x0C(12 in decimal) in big endian
        let unknown_2 = header_data.get_u32();
//        let unknown_2: u32 = u32::from_be_bytes([header_data[14], header_data[15], header_data[16], header_data[17]]); 
        if unknown_2 != 12 {
            error!("Unknown #2 was incorrect, expected: {}, received: {}", 12, unknown_2);
            return Err(());
        }

        // // An unknown value that varies but isn't unique enough to be an ID. Big endian
        let unknown_3 = header_data.get_u32();
//        let unknown_3: u32 = u32::from_be_bytes([header_data[18], header_data[19], header_data[20], header_data[21]]); 
        if unknown_3 != 5 {
            error!("Unknown #3 was not the expected value, expected: {}, received: {}", 5, unknown_3);
        }

        // Counter of packets sent this session. Little endian. Used in replies
        let session_sent_packets_counter = header_data.get_u32_le();
//        let session_sent_packets_counter: u32 = u32::from_le_bytes([header_data[22], header_data[23], header_data[24], header_data[25]]); 
        if session_sent_packets_counter < self.client_info.packet_counter {
            error!("Received session packet counter was less than what we have stored, stored value: {}, received value: {}", self.client_info.packet_counter, session_sent_packets_counter);
            return Err(());
        }

        self.client_info.packet_counter = session_sent_packets_counter;

        return Ok(());
    }

    async fn craft_header(&mut self) {

    }

    async fn send_auth_server_info(&mut self) {

        let mut server_info = frpg2_request_message::RequestQueryLoginServerInfoResponse::default();
        server_info.serverip = self.config.read().get_server_ip().to_string();
        server_info.port = self.config.read().get_auth_port() as i64;
        println!("Server IP: {}", server_info.serverip);
        let mut payload = BytesMut::with_capacity(server_info.encoded_len());
        server_info.encode(&mut payload).unwrap();
        println!("Len: {}", payload.len());

    }
}


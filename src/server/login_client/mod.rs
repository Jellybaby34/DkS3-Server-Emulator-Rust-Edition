use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::net::SocketAddr;
use std::fmt::Write;

use parking_lot::{Mutex, RwLock};
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;



use crate::Config;
use crate::server::RsaManager;
use crate::server::LogManager;

pub const CLIENT_TO_SERVER_HEADER: u8 = 26;
pub const SERVER_TO_CLIENT_HEADER: u8 = 0;

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
    log_manager: Arc<Mutex<LogManager>>
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
        log_manager: Arc<Mutex<LogManager>>
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
            client_info,
            log_manager
        }
    }

    fn log(&self, s: &str) {
        self.log_manager.lock().write(&format!("{0}: {1}", self.client_info.peer_addr, s));
    }

    ///// Command processing
    pub async fn process(&mut self) {
        loop {
            let mut header_data = [0; CLIENT_TO_SERVER_HEADER as usize];

            let r = self.stream_reader.read_exact(&mut header_data).await;

            match r {
                Ok(header_length) => {
                    let mut payload_data: [u8; 2000] = [0; 2000];
                    let payload_length = self.stream_reader.read(&mut payload_data).await.unwrap(); 

                    if self.process_header(&header_data, header_length, payload_length).await.is_err() {
                        self.log("Disconnecting");
                        break;
                    }

                    // The rust OpenSSL bindings don't let you pass buffer length
                    // so we need to slice the buffer so that it's the expected size
                    let payload_data_slice = &payload_data[0..payload_length];

                    let mut payload_decrypted: [u8; 2000] = [0; 2000];
                    let payload_length_decrypted = self.rsa_manager.read().rsa_decrypt( &payload_data_slice, &mut payload_decrypted);

                    let mut s = String::new();
                    for i in 0..payload_length_decrypted {
                        write!(&mut s, "{:02X} ", payload_decrypted[i]).expect("Unable to write")
                    }
                    println!("{}", s);

                    break;

                }
                Err(e) => {
                    let message = format!("Client disconnected: {}", &e);
                    self.log(&message);
                    break;
                }
            }
        }
    }

    async fn process_header(&mut self, header_data: &[u8], header_length: usize, payload_length: usize) -> Result<(), ()> {
        if header_length < CLIENT_TO_SERVER_HEADER.into() {
            self.log("Received header was shorter than expected");
            return Err(());
        }
        let total_packet_length: u32 = (header_length+payload_length) as u32;
        // Total packet length - 2 in big endian
        let packet_length_1_be: u16 = u16::from_be_bytes([header_data[0], header_data[1]]);
        if packet_length_1_be != (total_packet_length-2) as u16 {
            self.log("Packet length #1 was incorrect");
            return Err(());
        }
/*
        // Counter of total number of packets sent since game started, ?big endian
        let sent_packets_counter: u16 = u16::from_be_bytes([header_data[2], header_data[3]]); 
        if sent_packets_counter != (/**/).into() {
            self.log("Sent counter was incorrect");
            return Err(());
        }
*/
        // Always 0x00 0x00
        let unknown_1: u16 = u16::from_be_bytes([header_data[4], header_data[5]]); 
        if unknown_1 != 0 {
            self.log("Unknown #1 was incorrect");
            return Err(());
        }

        // Total packet length - 14 in big endian
        let packet_length_2a_be: u32 = u32::from_be_bytes([header_data[6], header_data[7], header_data[8], header_data[9]]); 
        if packet_length_2a_be != (total_packet_length-14) {
            self.log("Packet length #2A was incorrect");
            return Err(());
        }

        // Total packet length - 14 in big endian
        let packet_length_2b_be: u32 = u32::from_be_bytes([header_data[10], header_data[11], header_data[12], header_data[13]]); 
        if packet_length_2b_be != (total_packet_length-14) {
            self.log("Packet length #2B was incorrect");
            return Err(());
        }

        // Always 0x0C(12 in decimal) in big endian
        let unknown_2: u32 = u32::from_be_bytes([header_data[14], header_data[15], header_data[16], header_data[17]]); 
        if unknown_2 != 12 {
            self.log("Unknown #2 was incorrect");
            return Err(());
        }

        // // An unknown value that varies but isn't unique enough to be an ID. Big endian
        let unknown_3: u32 = u32::from_be_bytes([header_data[18], header_data[19], header_data[20], header_data[21]]); 
        if unknown_3 != 5 {
            self.log("Unknown #3 was not the expected value");
        }

        // Counter of packets sent this session. Little endian. Used in replies
        let session_sent_packets_counter: u32 = u32::from_le_bytes([header_data[22], header_data[23], header_data[24], header_data[25]]); 
        if session_sent_packets_counter < self.client_info.packet_counter {
            self.log("Received sent session packet counter value was less than what we have stored");
            return Err(());
        }

        self.client_info.packet_counter = session_sent_packets_counter;

        return Ok(());
    }
}


use async_trait::async_trait;
use prost::Message;
use rand::Rng;

use dks3_proto::frame::CipherMode;
use dks3_proto::msg::frpg2_request::{
    GetServiceStatus, GetServiceStatusResponse, RequestHandshake,
};

use crate::context::MatchmakingDb;
use crate::net;
use crate::net::server::{ConnectionHandler, TcpServer};
use crate::net::Connection;
use crate::Config;

pub struct AuthConnectionHandler {
    global_counter: u16,
    counter: u32,
}

impl Default for AuthConnectionHandler {
    fn default() -> Self {
        Self {
            global_counter: 0,
            counter: 0,
        }
    }
}

// @TODO: this code might be able to be shifted into [Connection]
impl AuthConnectionHandler {
    async fn write_data(&mut self, conn: &mut Connection, data: &[u8]) {}

    async fn write_message<M: Message>(&mut self, conn: &mut Connection, message: M) {
        net::message::write_message(conn, message, self.global_counter, self.counter).await
    }

    async fn read_message<M: Message + Default>(&mut self, conn: &mut Connection) -> M {
        let (message, global_counter, counter) = net::message::read_message(conn).await;

        self.global_counter = global_counter;
        self.counter = counter;

        message
    }
}

#[async_trait]
impl ConnectionHandler<MatchmakingDb> for AuthConnectionHandler {
    fn description() -> &'static str {
        "auth"
    }

    async fn run(&mut self, conn: &mut Connection, db: MatchmakingDb) {
        let handshake = self.read_message::<RequestHandshake>(conn).await;
        let cwc_key = handshake.aescwckey.as_bytes();

        conn.change_cipher_mode(CipherMode::aes128_cwc(cwc_key))
            .await;

        // Generate random 11 bytes, used as IV for CWC cipher on the client?
        let mut iv = rand::thread_rng().gen::<[u8; 11]>().to_vec();
        iv.resize(27, 0);

        self.write_data(conn, &iv).await;

        let status_req = self.read_message::<GetServiceStatus>(conn).await;
    }
}

pub fn create_auth_service(db: &MatchmakingDb) -> TcpServer<MatchmakingDb, AuthConnectionHandler> {
    let config = db.config();
    let bind_addr = format!("{}:{}", config.server_ip, config.auth_port);
    let inbound_cipher_mode = CipherMode::rsa_pkcs1_oeap(config.rsa_private_key.as_bytes());
    let outbound_cipher_mode = CipherMode::rsa_x931(config.rsa_private_key.as_bytes());
    let ciphers = (inbound_cipher_mode, outbound_cipher_mode);

    TcpServer::new(bind_addr, ciphers, db.clone())
}

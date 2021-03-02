use async_trait::async_trait;
use prost::Message;
use rand::Rng;
use bytes::BufMut;

use dks3_proto::frame::CipherMode;
use dks3_proto::msg::frpg2_request::{
    GetServiceStatus, GetServiceStatusResponse, RequestHandshake,
};

use crate::context::MatchmakingDb;
use crate::net;
use crate::net::server::{ConnectionHandler, TcpServer};
use crate::net::Connection;
use crate::Config;

use tracing::{error, info};

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
    async fn write_data(&mut self, conn: &mut Connection, data: &[u8]) {
        net::message::write_data(conn, data, self.global_counter, self.counter).await
    }

    async fn read_data(&mut self, conn: &mut Connection) -> bytes::BytesMut {
        let (data, global_counter, counter) = net::message::read_data(conn).await;

        self.global_counter = global_counter;
        self.counter = counter;

        data
    }

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
        let cwc_key = handshake.aescwckey.as_slice();

        info!("key = {}", hex::encode(cwc_key));

        conn.change_cipher_mode(CipherMode::aes128_cwc(cwc_key))
            .await;

        let init_block = [0u8; 16];
        self.write_data(conn, &init_block).await;

        let status_req = self.read_message::<GetServiceStatus>(conn).await;
        info!("steamid {}", status_req.steamid);

        let status_response = GetServiceStatusResponse {
            id: 2,
            steamid: "\x00".to_string(),
            unknownfield: 0,
            versionnum: 0,
        };

        self.write_message(conn, status_response).await;

        // Some kind of exchange between client/server
        // Client sends 8 bytes, server adds another 8 bytes then resends it
        // ?key exchange for something
        let mut unknown_16bytes = bytes::BytesMut::with_capacity(16);
        let client_8bytes = self.read_data(conn).await;
        unknown_16bytes.put(&client_8bytes[..]);
        let server_8bytes = rand::thread_rng().gen::<[u8; 8]>();
        unknown_16bytes.put(&server_8bytes[..]);

        info!("unknown16bytes = {}", hex::encode(&unknown_16bytes));
        self.write_data(conn, &unknown_16bytes[..]).await;

        // Here the client sends us their steam session ticket
        // We could try and validate it but the only struct I could find as a guide
        // is 8+ years old and is only good for a rough guide
        // https://github.com/SteamRE/SteamKit/blob/master/Resources/Structs/steam3_appticket.hsl
        // Size is 268 bytes (0x10C)
        let steam_ticket = self.read_data(conn).await;

        let mut ticket_steamid = [0u8; 8];
        ticket_steamid.copy_from_slice(&steam_ticket[28..36]);
        ticket_steamid.reverse();
        info!("Status req steamid: {}, ticket steamid: {}", status_req.steamid, hex::encode(ticket_steamid));

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

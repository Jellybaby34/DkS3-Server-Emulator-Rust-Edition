use async_trait::async_trait;
use bytes::BytesMut;
use prost::Message;
use tracing::info;

use dks3_proto::frame::{CipherMode, Frame};
use dks3_proto::msg::frpg2_request::RequestQueryLoginServerInfo;
use dks3_proto::msg::frpg2_request::RequestQueryLoginServerInfoResponse;

use crate::context::MatchmakingDb;
use crate::net::server::{ConnectionHandler, TcpServer};
use crate::net::Connection;
use crate::{net, Config};
use std::time::Duration;

pub struct LoginConnectionHandler {
    global_counter: u16,
    counter: u32,
}

impl Default for LoginConnectionHandler {
    fn default() -> Self {
        Self {
            global_counter: 0,
            counter: 0,
        }
    }
}

impl LoginConnectionHandler {
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
impl ConnectionHandler<MatchmakingDb> for LoginConnectionHandler {
    fn description() -> &'static str {
        "login"
    }

    async fn run(&mut self, conn: &mut Connection, context: MatchmakingDb) {
        let server_info_req = self.read_message::<RequestQueryLoginServerInfo>(conn).await;

        /* Could check steam ID, versionnum, etc. here */
        info!(steamid = %server_info_req.steamid, version = %server_info_req.versionnum, "Client connected");

        let config = context.config();
        let server_info = RequestQueryLoginServerInfoResponse {
            serverip: config.server_ip.clone(),
            port: config.auth_port.into(),
        };

        self.write_message(conn, server_info).await;

        tokio::time::sleep(Duration::from_millis(2000)).await;
    }
}

pub fn create_login_service(
    db: &MatchmakingDb,
) -> TcpServer<MatchmakingDb, LoginConnectionHandler> {
    let config = db.config();
    let bind_addr = format!("{}:{}", config.server_ip, config.login_port);
    let inbound_cipher_mode = CipherMode::rsa_pkcs1_oeap(config.rsa_private_key.as_bytes());
    let outbound_cipher_mode = CipherMode::rsa_x931(config.rsa_private_key.as_bytes());
    let ciphers = (inbound_cipher_mode, outbound_cipher_mode);

    TcpServer::new(bind_addr, ciphers, db.clone())
}

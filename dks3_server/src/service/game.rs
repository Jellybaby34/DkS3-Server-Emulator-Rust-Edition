use async_trait::async_trait;
use bytes::BytesMut;
use prost::Message;
use tracing::info;

use dks3_proto::frame::{CipherMode, Frame};

use crate::context::MatchmakingDb;
use crate::net::Connection;
use crate::{net, Config};
use std::time::Duration;

use crate::net::server::{ConnectionHandler, UdpServer};

pub struct GameConnectionHandler {
}

impl Default for GameConnectionHandler {
    fn default() -> Self {
        Self {
        }
    }
}

impl GameConnectionHandler {
    async fn write_message<M: Message>(&mut self, conn: &mut Connection, message: M) {
        unimplemented!();
    }

    async fn read_message<M: Message + Default>(&mut self, conn: &mut Connection) -> M {
        unimplemented!();
    }
}

#[async_trait]
impl ConnectionHandler<MatchmakingDb> for GameConnectionHandler {
    fn description() -> &'static str {
        "game"
    }

    async fn run(&mut self, conn: &mut Connection, context: MatchmakingDb) {
        unimplemented!();
    }
}

pub fn create_game_service(
    db: &MatchmakingDb,
) -> UdpServer<MatchmakingDb, GameConnectionHandler> {
    let config = db.config();
    let bind_addr = format!("{}:{}", config.server_ip, config.game_port);

    UdpServer::new(bind_addr, db.clone())
}

use std::clone::Clone;
use std::io;
use std::net::ToSocketAddrs;

use parking_lot::RwLock;
use tokio::net::TcpListener;
use tokio::runtime;
use tracing::{debug, error, info, span, trace, warn, Level};

use super::LoginConnectionHandler;
use crate::server::RsaManager;
use crate::Config;

pub struct LoginServer {
    config: Config,
    rsa_manager: RsaManager,
}

impl LoginServer {
    pub fn new(config: Config, rsa_manager: RsaManager) -> LoginServer {
        LoginServer {
            config,
            rsa_manager,
        }
    }

    pub async fn start(self) -> Result<(), std::io::Error> {
        info!("Starting login server");

        // Parse host address and login port
        let str_addr =
            self.config.get_server_ip().clone() + ":" + &self.config.get_login_port().to_string();
        let mut addr = str_addr.to_socket_addrs().map_err(|e| {
            io::Error::new(e.kind(), format!("{} is not a valid address", &str_addr))
        })?;
        let addr = addr.next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::AddrNotAvailable,
                format!("{} is not a valid address", &str_addr),
            )
        })?;

        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| io::Error::new(e.kind(), format!("Error binding to <{}>: {}", &addr, e)))
            .unwrap();
        info!("Now waiting for connections on <{}>", &addr);

        loop {
            let (stream, peer_addr) = match listener.accept().await {
                Err(e) => {
                    info!("Accept failed with: {}", e);
                    continue;
                }
                Ok(result) => result,
            };

            info!("New client from {}", peer_addr);

            let config = self.config.clone();

            tokio::spawn(async move {
                let mut client = LoginConnectionHandler::new(stream, config);

                client.await.run().await;
                Ok(()) as io::Result<()>
            });
        }
    }
}

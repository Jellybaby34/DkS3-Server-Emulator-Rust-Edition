use std::sync::Arc;
use std::net::ToSocketAddrs;
use std::io;
use tokio::net::TcpListener;
use tokio::runtime;
use parking_lot::RwLock;

use tracing::{debug, error, info, span, trace, warn, Level};

use crate::Config;
use crate::server::rsa_manager::RsaManager;
mod login_client;
use login_client::LoginClient;

pub struct LoginServer {
    config: Arc<RwLock<Config>>,
    tokio_runtime: Arc<runtime::Runtime>,
    rsa_manager: Arc<RwLock<RsaManager>>
}

impl LoginServer {
    pub fn new(config: Arc<RwLock<Config>>, tokio_runtime: Arc<runtime::Runtime>, rsa_manager: Arc<RwLock<RsaManager>>) -> LoginServer {
        LoginServer {
            config,
            tokio_runtime,
            rsa_manager
        }
    }

    pub async fn start(self) -> Result<(), std::io::Error> {
        
        info!("Starting login server");

        // Parse host address and login port
        let str_addr = self.config.read().get_server_ip().clone() + ":" + &self.config.read().get_login_port().to_string();
        let mut addr = str_addr.to_socket_addrs().map_err(|e| io::Error::new(e.kind(), format!("{} is not a valid address", &str_addr)))?;
        let addr = addr.next().ok_or_else(|| io::Error::new(io::ErrorKind::AddrNotAvailable, format!("{} is not a valid address", &str_addr)))?;

        let listener = TcpListener::bind(&addr).await.map_err(|e| io::Error::new(e.kind(), format!("Error binding to <{}>: {}", &addr, e))).unwrap();
        info!("Now waiting for connections on <{}>", &addr);

        loop {
            let accept_result = listener.accept().await;
            if let Err(e) = accept_result {
                info!("Accept failed with: {}", e);
                continue;
            }

            let (stream, peer_addr) = accept_result.unwrap();
            info!("New client from {}", peer_addr);

            let config = self.config.clone();
            let rsa_client = self.rsa_manager.clone();

            let fut_client = async move {
                let mut client = LoginClient::new(peer_addr, stream, config, rsa_client).await;
                client.process().await;
                Ok(()) as io::Result<()>
            };

            self.tokio_runtime.spawn(fut_client);
        }

    }
}

/*

        let fut_server = async move {

            let listener = TcpListener::bind(&addr).await.map_err(|e| io::Error::new(e.kind(), format!("Error binding to <{}>: {}", &addr, e))).unwrap();
            self.log(&format!("Now waiting for connections on <{}>", &addr));

            loop {
                let accept_result = listener.accept().await;
                if let Err(e) = accept_result {
                    self.log(&format!("Accept failed with: {}", e));
                    continue;
                }

                let (stream, peer_addr) = accept_result.unwrap();
//                stream.set_keepalive(Some(std::time::Duration::new(30, 0))).unwrap();
                self.log(&format!("New client from {}", peer_addr));

                let config = self.config.clone();
                let rsa_client = self.rsa_manager.clone();
                let log_client = self.log_manager.clone();

                let fut_client = async move {
//                    let payload = b"Lmao";
//                    let mut payload_buffer: [u8; 256] = [0; 256];
//                    rsa_client.read().rsa_encrypt(payload, &mut payload_buffer);
//                    stream_example.write(&payload_buffer).await?;
                    let mut client = LoginClient::new(peer_addr, stream, config, rsa_client, log_client).await;
                    client.process().await;
                    Ok(()) as io::Result<()>
                };

                runtime.spawn(fut_client);
            }

        };
*/
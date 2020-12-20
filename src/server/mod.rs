use std::sync::Arc;
use std::net::ToSocketAddrs;
use std::io::{self};

use tokio::net::TcpListener;
use tokio::runtime;

use parking_lot::{Mutex, RwLock};

use crate::Config;
mod log_manager;
use log_manager::LogManager;
mod rsa_manager;
use rsa_manager::RsaManager;
pub mod login_client;
use login_client::LoginClient;

use tracing::{debug, error, info, span, trace, warn, Level};

pub struct Server {
    config: Arc<RwLock<Config>>,
    log_manager: Arc<Mutex<LogManager>>,
    rsa_manager: Arc<RwLock<RsaManager>>
}

impl Server {
    pub fn new(config_inst: Config) -> Server {
        let config = Arc::new(RwLock::new(config_inst));
        let log_manager = Arc::new(Mutex::new(LogManager::new()));
        let rsa_manager = Arc::new(RwLock::new(RsaManager::new(config.clone())));

        Server {
            config,
            log_manager,
            rsa_manager
        }
    }

    fn log(&self, s: &str) {
        self.log_manager.lock().write(&format!("Server: {}", s));
    }

    pub fn start(&mut self) -> Result<(), std::io::Error> {
        
        info!("Starting server");
        self.log("Starting server instance");

        // Parse host address
        let str_addr = self.config.read().get_server_ip().clone() + ":" + &self.config.read().get_login_port().to_string();
        let mut addr = str_addr.to_socket_addrs().map_err(|e| io::Error::new(e.kind(), format!("{} is not a valid address", &str_addr)))?;
        let addr = addr
            .next()
            .ok_or_else(|| io::Error::new(io::ErrorKind::AddrNotAvailable, format!("{} is not a valid address", &str_addr)))?;


        // Setup Tokio
        let runtime = runtime::Builder::new_multi_thread()
        .enable_io()
        .build()?;
        
        let fut_server = async {

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

        let res = runtime.block_on(fut_server);
        Ok(())

    }



}
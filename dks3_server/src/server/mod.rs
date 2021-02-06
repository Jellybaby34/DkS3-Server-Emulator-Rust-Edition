use std::sync::Arc;
use tokio::runtime;
use parking_lot::RwLock;

use tracing::{error, info};

use crate::Config;
mod rsa_manager;
use rsa_manager::RsaManager;
mod login_server;
use login_server::LoginServer;

pub struct ServerMaster {
    config: Arc<RwLock<Config>>,
    pub tokio_runtime: Arc<runtime::Runtime>,
    rsa_manager: Arc<RwLock<RsaManager>>
}

impl ServerMaster {
    pub fn new(config_inst: Config) -> ServerMaster {
        let config = Arc::new(RwLock::new(config_inst));
        let tokio_runtime = Arc::new(runtime::Builder::new_multi_thread().enable_io().build().unwrap());
        let rsa_manager = Arc::new(RwLock::new(RsaManager::new(config.clone())));

        ServerMaster {
            config,
            tokio_runtime,
            rsa_manager
        }
    }

    pub async fn hello() {
        loop {
//            info!("Hello");
        }
    }
    pub async fn start(self) -> Result<(), std::io::Error> {
        
        info!("Starting server instances...");

        let config = self.config.clone();
        let tokio_runtime = self.tokio_runtime.clone();
        let rsa_manager = self.rsa_manager.clone();
        let login_server_inst = LoginServer::new(config, tokio_runtime, rsa_manager);

        let config = self.config.clone();
        let tokio_runtime = self.tokio_runtime.clone();
        let rsa_manager = self.rsa_manager.clone();
//        let fut_login2 = self.tokio_runtime.spawn(ServerMaster::hello());

        tokio::select! {
            r = self.tokio_runtime.spawn(login_server_inst.start()) => {
                error!("Error login server");
            },
            r = self.tokio_runtime.spawn(ServerMaster::hello()) => {
                
            },
        };

        Ok(())

    }
}



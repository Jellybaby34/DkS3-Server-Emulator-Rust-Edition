extern crate config;

use tracing::{error, info};

mod server;
use server::ServerMaster;

pub struct Config {
    server_ip: String,
    login_port: u16,
    auth_port: u16,
    game_port: u16,
    rsa_private_key: String
}

impl Config {
    pub fn new(config_file: config::Config) -> Config {
        Config {
            server_ip: config_file.get_str("server_ip").expect("Could not read server_ip from config file"),
            login_port: config_file.get_int("login_port").expect("Could not read login_port from config file") as u16,
            auth_port: config_file.get_int("auth_port").expect("Could not read auth_port from config file") as u16,
            game_port: config_file.get_int("game_port").expect("Could not read game_port from config file") as u16,
            rsa_private_key: config_file.get_str("rsa_private_key").expect("Could not read rsa_private_key from config file")
        }
    }

    pub fn get_server_ip(&self) -> &String {
        return &self.server_ip;
    }

    pub fn get_login_port(&self) -> u16 {
        return self.login_port;
    }

    pub fn get_auth_port(&self) -> u16 {
        return self.auth_port;
    }

    pub fn get_game_port(&self) -> u16 {
        return self.game_port;
    }

    pub fn get_rsa_private_key(&self) -> &String {
        return &self.rsa_private_key;
    }
}

fn main() {
    // Set up logging things
    // Should really add the module that logs to file
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
    .with_max_level(tracing::Level::TRACE)
    .without_time()
    .with_target(true)
    .with_ansi(true)
    .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Setting default subscriber failed!");

    info!("Starting Dark Souls 3 Server Emulator");
    info!("Written by /u/TheSpicyChef");
    info!("Don't expect perfection because i've never used rust before :lmao:");

    // Read config settings from the "Settings.toml" file
    let mut settings = config::Config::default();
    settings.merge(config::File::with_name("Settings")).unwrap();
    let config_inst = Config::new(settings);

    // Create our "ServerMaster" instance that will start and handle the other instances
    let server_inst = ServerMaster::new(config_inst);

    // Start the ServerMaster and block on it.
    let tokio = server_inst.tokio_runtime.clone();
    let res = tokio.block_on(server_inst.start());

    if let Err(e) = res {
        error!("Server terminated with error: {}", e);
    } else {
        error!("Server terminated normally");
    }
    
}

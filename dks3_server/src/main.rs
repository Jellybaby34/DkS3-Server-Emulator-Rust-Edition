extern crate config;
extern crate tokio;

use std::clone::Clone;

use tracing::{error, info};

use crate::login::LoginServer;
use crate::server::RsaManager;

mod login;
mod net;
mod server;

#[derive(Clone)]
pub struct Config {
    server_ip: String,
    login_port: u16,
    auth_port: u16,
    game_port: u16,
    rsa_private_key: String,
}

impl Config {
    pub fn new(config_file: config::Config) -> Config {
        Config {
            server_ip: config_file
                .get_str("server_ip")
                .expect("Could not read server_ip from config file"),
            login_port: config_file
                .get_int("login_port")
                .expect("Could not read login_port from config file")
                as u16,
            auth_port: config_file
                .get_int("auth_port")
                .expect("Could not read auth_port from config file") as u16,
            game_port: config_file
                .get_int("game_port")
                .expect("Could not read game_port from config file") as u16,
            rsa_private_key: config_file
                .get_str("rsa_private_key")
                .expect("Could not read rsa_private_key from config file"),
        }
    }

    pub fn get_server_ip(&self) -> &String {
        &self.server_ip
    }

    pub fn get_login_port(&self) -> u16 {
        self.login_port
    }

    pub fn get_auth_port(&self) -> u16 {
        self.auth_port
    }

    pub fn get_game_port(&self) -> u16 {
        self.game_port
    }

    pub fn get_rsa_private_key(&self) -> &String {
        &self.rsa_private_key
    }
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, Error>;

#[tokio::main]
async fn main() -> Result<()> {
    // Set up logging things
    // Should really add the module that logs to file
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::TRACE)
        .without_time()
        .with_target(true)
        .with_ansi(true)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Setting default subscriber failed!");

    info!("Starting Dark Souls 3 Server Emulator");
    info!("Written by /u/TheSpicyChef");
    info!("Don't expect perfection because i've never used rust before :lmao:");

    // Read config settings from the "Settings.toml" file
    let mut settings = config::Config::default();
    settings.merge(config::File::with_name("Settings")).unwrap();
    let config = Config::new(settings);

    let rsa_manager = RsaManager::new(config.clone());
    let login_server = LoginServer::new(config.clone(), rsa_manager);

    tokio::try_join!(login_server.start())?;

    Ok(())
}

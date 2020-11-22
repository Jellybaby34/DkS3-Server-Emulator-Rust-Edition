extern crate config;

mod server;
use server::Server;

//use std::collections::HashMap;

pub struct Config {
    server_ip: String,
    login_port: u16,
    auth_port: u16,
    game_port: u16,
    rsa_public_key: String,
    rsa_private_key: String
}

impl Config {
    pub fn new(config_file: config::Config) -> Config {
        Config {
            server_ip: config_file.get_str("server_ip").expect("Could not read server_ip from config file"),
            login_port: config_file.get_int("login_port").expect("Could not read login_port from config file") as u16,
            auth_port: config_file.get_int("auth_port").expect("Could not read auth_port from config file") as u16,
            game_port: config_file.get_int("game_port").expect("Could not read game_port from config file") as u16,
            rsa_public_key: config_file.get_str("rsa_public_key").expect("Could not read rsa_public_key from config file"),
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

    pub fn get_rsa_public_key(&self) -> &String {
        return &self.rsa_public_key;
    }

    pub fn get_rsa_private_key(&self) -> &String {
        return &self.rsa_private_key;
    }
}

fn main() {
    println!("[main] Starting Dark Souls 3 Server Emulator");
    println!("[main] Written by /u/TheSpicyChef");
    println!("[main] Don't expect perfection because i've never used rust before :lmao:");

    let mut settings = config::Config::default();
    settings.merge(config::File::with_name("Settings")).unwrap();
    let config_inst = Config::new(settings);

    let mut server_inst = Server::new(config_inst);
    
    if let Err(e) = server_inst.start() {
        println!("Server terminated with error: {}", e);
    } else {
        println!("Server terminated normally");
    }

}
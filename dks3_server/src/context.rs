use std::sync::Arc;
use std::collections::HashMap;

use crate::Config;
use crate::net::UdpConnection;
use std::sync::RwLock;

#[derive(Debug, Clone)]
pub struct MatchmakingDb {
    config: Config,
    shared: Arc<MatchmakingDbShared>,
}

impl MatchmakingDb {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            shared: Default::default(),
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }
}

#[derive(Default, Debug)]
pub struct MatchmakingState {}

#[derive(Default, Debug)]
pub struct MatchmakingDbShared {
    state: RwLock<MatchmakingState>,
}

//
// UDP HERE
//

pub struct ConnectionDb {
    config: Config,
    shared: Arc<ConnectionDbShared>,
}

impl ConnectionDb {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            shared: Default::default(),
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn read(&self, key: &u64) -> Arc<UdpConnection> {
        let test = self.shared.state.read().unwrap();
        let conn = test.hashmap.get(key).unwrap().clone();

        conn
    }

    pub fn write(&self) {

    }
}

#[derive(Default)]
pub struct ConnectionState {
    hashmap: HashMap<u64, Arc<UdpConnection>>,
}

#[derive(Default)]
pub struct ConnectionDbShared {
    state: RwLock<ConnectionState>,
}

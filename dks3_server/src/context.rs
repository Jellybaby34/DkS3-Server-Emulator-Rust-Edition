use std::sync::Arc;

use crate::Config;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct MatchmakingDb {
    config: Config,
    shared: Arc<Shared>,
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
pub struct Shared {
    state: RwLock<MatchmakingState>,
}

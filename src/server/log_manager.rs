use chrono::Utc;

use tracing::{debug, error, info, span, trace, warn, Level};

// To be expanded to log to file
pub struct LogManager {}

impl LogManager {
    pub fn new() -> LogManager {
        LogManager {}
    }

    pub fn write(&self, s: &str) {
        info!("{}", s);
        println!("{}: {}", Utc::now().format("%Y-%m-%d %H:%M:%S"), s);
    }
}
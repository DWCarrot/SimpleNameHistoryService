use std::net::SocketAddr;
use std::path::PathBuf;

use serde::Serialize;
use serde::Deserialize;

#[derive(Debug,Serialize,Deserialize)]
pub struct ServerConfig {
    pub address: SocketAddr,
    pub static_files: Option<PathBuf>,
}

impl Default for ServerConfig {

    fn default() -> Self {
        Self {
            address: SocketAddr::from(([127, 0, 0, 1], 6080)),
            static_files: None
        }
    }
}
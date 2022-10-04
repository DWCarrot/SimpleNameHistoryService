use std::net::SocketAddr;
use std::path::PathBuf;

use serde::Serialize;
use serde::Deserialize;

use crate::client::config::ClientConfig;
use crate::server::config::ServerConfig;
use crate::storage::config::DatabaseConfig;


#[derive(Debug,Default,Serialize,Deserialize)]
pub struct Config {
    pub client: ClientConfig,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
}






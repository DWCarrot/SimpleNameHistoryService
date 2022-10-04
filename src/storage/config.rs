use std::time::Duration;

use serde::Serialize;
use serde::Deserialize;

use crate::utils::configfile;


#[derive(Debug,Serialize,Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(with="crate::utils::duration_fmt")]
    pub timeout: Duration,
    #[serde(with="crate::utils::duration_fmt")]
    pub pool_timeout: Duration,
    pub pool_max_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self { 
            url: String::from("sqlite://data.db"),
            timeout: Duration::from_secs(16),
            pool_timeout: Duration::from_secs(32),
            pool_max_connections: 2,
        }
    }
}
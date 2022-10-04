use std::net::SocketAddr;
use std::time::Duration;
use std::time::SystemTime;

use serde::Serialize;
use serde::Deserialize;

#[derive(Debug,Serialize,Deserialize)]
pub struct ClientConfig {
    pub pool_size: usize,
    #[serde(with="crate::utils::duration_fmt")]
    pub timeout: Duration,
    pub user_agent: Option<String>,
    pub proxies: Vec<ProxyConfig>,
    pub use_cache: UseCacheConfig,
}

impl Default for ClientConfig {

    fn default() -> Self {
        Self { 
            pool_size: 8, 
            timeout: Duration::from_millis(5000),
            user_agent: None,
            proxies: Vec::new(),
            use_cache: UseCacheConfig::default()
        }
    }
}


#[derive(Debug,Serialize,Deserialize)]
pub struct ProxyConfig {
    pub address: SocketAddr,
    pub authorization: Option<ProxyAuthorizationConfig>,
}

#[derive(Debug,Serialize,Deserialize)]
pub struct ProxyAuthorizationConfig {
    pub user_name: String,
    pub password: String,
}


#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct UseCacheConfig {
    #[serde(with="crate::utils::duration_fmt")]
    pub unchanged: Duration,
    #[serde(with="crate::utils::duration_fmt")]
    pub changed: Duration,
}

impl Default for UseCacheConfig {

    fn default() -> Self {
        Self { 
            unchanged: Duration::from_secs(12 * 3600), 
            changed: Duration::from_secs(30 * 24 * 3600)
        }
    }
}
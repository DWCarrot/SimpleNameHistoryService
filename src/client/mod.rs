use std::sync::Arc;
use std::time::Duration;

use headers::Authorization;
use hyper::Client;
use hyper::Method;
use hyper::Request;
use hyper::Body;
use hyper::StatusCode;
use hyper::body;
use hyper::body::Buf;
use hyper::client::ResponseFuture;
use hyper::client::connect::Connect;
use hyper::header;
use hyper::http::HeaderValue;
use hyper_proxy::Intercept;
use hyper_proxy::Proxy;
use hyper_proxy::ProxyConnector;
use hyper_tls::HttpsConnector;
use uuid::Uuid;


use self::config::ClientConfig;
use self::config::ProxyConfig;
use self::data::Profile;

pub mod data;
pub mod config;

trait GeneralClient {
    fn request(&self, req: Request<Body>) -> ResponseFuture;
}


struct ClientWrapper<C> {
    inner: Client<C, Body>,
    user_agent: Option<HeaderValue>,
}

impl<C: Connect + Clone + Send + Sync + 'static> GeneralClient for ClientWrapper<C> {

    fn request(&self, mut req: Request<Body>) -> ResponseFuture {
        if let Some(ref user_agent) = self.user_agent {
            req.headers_mut().insert(header::USER_AGENT, user_agent.clone());
        }
        self.inner.request(req)
    }
}


pub enum JsonRequesterError {
    Deserialize(serde_json::Error),
    Hyper(hyper::Error),
    StatusCode(StatusCode),
}

impl From<serde_json::Error> for JsonRequesterError {

    fn from(e: serde_json::Error) -> Self {
        Self::Deserialize(e)
    }
}

impl From<hyper::Error> for JsonRequesterError {

    fn from(e: hyper::Error) -> Self {
        Self::Hyper(e)
    }
}


#[derive(Clone)]
pub struct MojangAPIRequester {
    client: Arc<dyn GeneralClient + Send + Sync>
}

impl MojangAPIRequester {
    
    pub fn new(config: &ClientConfig) -> Self {
        let mut builder = Client::builder();
        builder.pool_idle_timeout(config.timeout);
        builder.pool_max_idle_per_host(config.pool_size);
        let connector = HttpsConnector::new();
        let user_agent = config.user_agent.as_ref().and_then(|s| HeaderValue::from_str(s).ok());
        let proxies = config.proxies.iter()
            .filter_map(build_proxy)
            .collect::<Vec<_>>();
        let client = if !proxies.is_empty() {
            let mut proxy_connector = ProxyConnector::unsecured(connector);
            for proxy in proxies {
                proxy_connector.add_proxy(proxy);
            }
            let inner = builder.build(proxy_connector);
            Arc::new(ClientWrapper {inner, user_agent}) as Arc<(dyn GeneralClient + Send + Sync + 'static)>
        } else {
            let inner = builder.build(connector);
            Arc::new(ClientWrapper {inner, user_agent}) as Arc<(dyn GeneralClient + Send + Sync + 'static)>
        };
        Self {
            client
        } 
    }

    pub async fn request_profile(&self, uuid: &Uuid) -> Result<Profile, JsonRequesterError> {
        let req = Request::builder()
            .uri(format!("https://sessionserver.mojang.com/session/minecraft/profile/{}", uuid))
            .method(Method::GET)
            .body(Body::empty())
            .unwrap();
        let resp = self.client.request(req).await?;
        let status_code = resp.status();
        if status_code == StatusCode::OK {
            let data = body::aggregate(resp.into_body()).await?;
            let profile = serde_json::from_reader(data.reader())?;
            Ok(profile)
        } else {
            Err(JsonRequesterError::StatusCode(status_code))
        }
        
    }
}


fn build_proxy(proxy_cfg: &ProxyConfig) -> Option<Proxy> {
    let url_str = format!("http://{}", proxy_cfg.address);
    match url_str.parse() {
        Ok(url) => {
            let mut proxy = Proxy::new(Intercept::Https, url);
            if let Some(auth) = &proxy_cfg.authorization {
                proxy.set_authorization(Authorization::basic(auth.user_name.as_str(), auth.password.as_str()));
            }
            Some(proxy)
        }
        Err(e) => {
            tracing::warn!("ignore invalid proxy {:?}: {}", url_str, e);
            None
        }
    }
    
}


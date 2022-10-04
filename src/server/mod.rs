use std::convert::Infallible;
use std::sync::Arc;

use hyper::Body;
use hyper::Response;
use sqlx::database;
use uuid::Uuid;
use warp::Filter;
use warp::Rejection;
use warp::fs::File;

use crate::client::MojangAPIRequester;
use crate::client::config::UseCacheConfig;
use crate::config::Config;
use crate::storage::NameHistoryDatabase;

use self::config::ServerConfig;

pub mod config;
pub mod namehistory;

static ROOT_INFO: &'static [u8] = b"Hyper Warp Server";

async fn ctrl_c_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to register CTRL+C signal handler");
}


pub async fn server(config: &Config) {
    let requester = MojangAPIRequester::new(&config.client);
    tracing::info!("requester running");
    let database = match NameHistoryDatabase::init(&config.database).await {
        Ok(v) => {
            tracing::info!("database linked @{}", config.database.url.as_str());
            v
        },
        Err(e) => {
            tracing::error!("database link error @{}: {}", config.database.url.as_str(), e);
            return ;
        }
    };
    let addr = config.server.address.clone();
    let root = warp::path::end()
        .map(|| { Response::new(Body::from(ROOT_INFO)) })
        .boxed();
    
    let static_files = if let Some(static_files_root) = &config.server.static_files {
        warp::path("static").and(warp::fs::dir(static_files_root.clone())).boxed()
    } else {
        warp::path("static").and_then(reject_file).boxed()
    };
    let use_cache_config = Arc::new(config.client.use_cache.clone());
    let name_history = warp::path("user").and(warp::path("profiles")).and(warp::path::param::<Uuid>()).and(warp::path("names")).and(warp::path::end())
        .and(Context::new_in_filter(requester.clone(), database.clone(), use_cache_config.clone()))
        .and_then(namehistory::handle_get_name_history)
        .boxed();

    let get_router = warp::get()
        .and(root.or(name_history).or(static_files))
        .with(warp::trace::request());
        // TODO: change with as better log

    let (addr, server) = warp::serve(get_router).bind_with_graceful_shutdown(addr, ctrl_c_signal());
    tracing::info!("server started @{}", &addr);

    server.await;
    tracing::info!("server stopped");
    database.close().await;
    tracing::info!("database closed");
}


#[derive(Clone)]
pub struct Context {
    pub(crate) requester: MojangAPIRequester,
    pub(crate) database: NameHistoryDatabase,
    pub(crate) use_cache_config: Arc<UseCacheConfig>,
}

impl Context {
    
    pub fn new_in_filter(requester: MojangAPIRequester, database: NameHistoryDatabase, use_cache_config: Arc<UseCacheConfig>) -> impl Filter<Extract = (Self, ), Error = Infallible> + Clone {
        warp::any().map(move || Context { requester: requester.clone(), database: database.clone(), use_cache_config: use_cache_config.clone() })
    }
}


pub(crate) async fn reject_file() -> Result<File, Rejection> {
    Err(warp::reject())
}

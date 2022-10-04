use crate::utils::configfile::ConfigFile;

mod storage;
mod client;
mod server;
mod config;
mod utils;

fn main() {

    let level = if let Ok(s) = std::env::var("LOG_LEVEL") {
        match s.as_str() {
            "TRACE" | "trace" => tracing::Level::TRACE,
            "DEBUG" | "debug" => tracing::Level::DEBUG,
            "INFO" | "info" => tracing::Level::INFO,
            "WARN" | "warn" => tracing::Level::WARN,
            "ERROR" | "error" => tracing::Level::ERROR,
            _ => tracing::Level::INFO
        }
    } else {
        tracing::Level::INFO
    };

    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(level)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    let (cfg, c): (ConfigFile<config::Config>, bool) = ConfigFile::new_nearby("config.json").unwrap();
    println!("{:?}", cfg.data());
    if c {
        return;
    }
    
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(server::server(&cfg.data()));
    
}



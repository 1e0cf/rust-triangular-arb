use dotenvy::dotenv;
use once_cell::sync::Lazy;
use std::env;

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    dotenv().ok();

    Config::init()
});

#[derive(Debug)]
pub(crate) struct Config {
    pub redis_url: String,
    pub metrics_addr: String,
}

impl Config {
    pub fn init() -> Self {
        let redis_url = env::var("REDIS_URL").unwrap();
        let metrics_addr = env::var("METRICS_ADDR").unwrap_or("0.0.0.0:9090".to_string());
        Self {
            redis_url,
            metrics_addr,
        }
    }
}

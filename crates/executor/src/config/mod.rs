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
    pub api_key: String,
    pub api_secret: String,
}

impl Config {
    pub fn init() -> Self {
        let redis_url = env::var("REDIS_URL").unwrap();
        let metrics_addr = env::var("METRICS_ADDR").unwrap_or("0.0.0.0:9090".to_string());
        let api_key = env::var("API_KEY").unwrap();
        let api_secret = env::var("API_SECRET").unwrap();
        Self {
            redis_url,
            metrics_addr,
            api_key,
            api_secret,
        }
    }
}

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
}

impl Config {
    pub fn init() -> Self {
        let redis_url = env::var("REDIS_URL").unwrap();
        Self { redis_url }
    }
}

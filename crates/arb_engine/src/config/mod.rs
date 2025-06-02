use dotenvy::dotenv;
use once_cell::sync::Lazy;
use rust_decimal::Decimal;
use std::env;
use std::str::FromStr;

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    dotenv().ok();

    Config::init()
});

#[derive(Debug)]
pub(crate) struct Config {
    pub redis_url: String,
    pub metrics_addr: String,
    pub min_profit_perc: Decimal,
    pub cycle_volume: Decimal,
    pub fee_percent: Decimal,
    pub stablecoins: Vec<String>
}

impl Config {
    pub fn init() -> Self {
        let redis_url = env::var("REDIS_URL").unwrap();
        let metrics_addr = env::var("METRICS_ADDR").unwrap_or("0.0.0.0:9090".to_string());
        let min_profit_perc =
            Decimal::from_str(env::var("MIN_PROFIT_PERC").unwrap().as_str()).unwrap();
        let cycle_volume = Decimal::from_str(env::var("CYCLE_VOLUME").unwrap().as_str()).unwrap();
        let fee_percent = Decimal::from_str(env::var("FEE_PERCENT").unwrap().as_str()).unwrap();
        let stablecoins: Vec<String> = env::var("STABLECOINS").unwrap().split(',').map(|s| s.to_string()).collect();
        Self {
            redis_url,
            metrics_addr,
            min_profit_perc,
            cycle_volume,
            fee_percent,
            stablecoins
        }
    }
}

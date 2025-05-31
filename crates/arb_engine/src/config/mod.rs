use dotenvy::dotenv;
use once_cell::sync::Lazy;
use std::env;
use std::str::FromStr;
use rust_decimal::Decimal;

pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    dotenv().ok();

    Config::init()
});

#[derive(Debug)]
pub(crate) struct Config {
    pub redis_url: String,
    pub min_profit_perc: Decimal,
    pub max_order_vol: Decimal,
    pub fee: Decimal
}

impl Config {
    pub fn init() -> Self {
        let redis_url = env::var("REDIS_URL").unwrap();
        let min_profit_perc = Decimal::from_str(env::var("MIN_PROFIT_PERC").unwrap().as_str()).unwrap();
        let max_order_vol = Decimal::from_str(env::var("MAX_ORDER_VOL").unwrap().as_str()).unwrap();
        let fee = Decimal::from_str(env::var("FEE").unwrap().as_str()).unwrap();
        Self { redis_url, min_profit_perc, max_order_vol, fee }
    }
}

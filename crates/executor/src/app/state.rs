use crate::config::CONFIG;
use binance_spot_connector_rust::http::Credentials;
use binance_spot_connector_rust::hyper::BinanceHttpClient;
use hyper::client::HttpConnector;
use hyper_tls::HttpsConnector;
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use std::sync::Arc;


pub type BinanceClient = BinanceHttpClient<HttpsConnector<HttpConnector>>;
pub struct AppState {
    pub redis: ConnectionManager,
    pub binance_client: Arc<BinanceClient>,
}
impl AppState {
    pub async fn init() -> anyhow::Result<Self> {
        let r = redis::Client::open(CONFIG.redis_url.as_str())?;
        let _: String = r.get_multiplexed_async_connection().await?.ping().await?; // Check if redis available
        let redis = ConnectionManager::new(r).await?;
        let binance_client = Arc::new(BinanceHttpClient::default().credentials(
            Credentials::from_hmac(CONFIG.api_key.as_str(), CONFIG.api_secret.as_str()),
        ));
        Ok(Self {
            redis,
            binance_client,
        })
    }
}

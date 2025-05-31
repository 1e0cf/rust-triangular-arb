use crate::config::CONFIG;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;

pub struct AppState {
    pub redis: ConnectionManager,
}
impl AppState {
    pub async fn init() -> anyhow::Result<Self> {
        let mut r = redis::Client::open(CONFIG.redis_url.as_str())?;
        let _: String = r.get_multiplexed_async_connection().await?.ping().await?; // Check if redis available
        let redis = ConnectionManager::new(r).await?;
        Ok(Self { redis })
    }
}

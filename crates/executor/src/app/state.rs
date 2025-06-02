use crate::config::CONFIG;
use redis::AsyncCommands;
use redis::aio::ConnectionManager;

pub struct AppState {
    pub redis: ConnectionManager,
}
impl AppState {
    pub async fn init() -> anyhow::Result<Self> {
        let r = redis::Client::open(CONFIG.redis_url.as_str())?;
        let _: String = r.get_multiplexed_async_connection().await?.ping().await?; // Check if redis available
        let redis = ConnectionManager::new(r).await?;
        Ok(Self { redis })
    }
}

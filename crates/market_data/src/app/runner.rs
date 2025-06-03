use crate::app::AppState;
use crate::app::tasks::depth_ws_connections_pool;
use binance_connector::fetch_exchange_info;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use crate::app::constants::STREAMS_PER_CONNECTION;

pub async fn run(ctx: CancellationToken) -> anyhow::Result<()> {
    let state = Arc::new(AppState::init().await?);
    let symbols: Vec<String> = fetch_exchange_info()
        .await?
        .into_iter()
        .map(|pair_info| pair_info.symbol.to_lowercase())
        .collect();
    depth_ws_connections_pool(ctx, symbols, STREAMS_PER_CONNECTION, state.redis.clone()).await
}

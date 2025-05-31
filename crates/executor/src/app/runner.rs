use tokio_util::sync::CancellationToken;
use crate::app::{tasks, AppState};

pub async fn run(ctx: CancellationToken) -> anyhow::Result<()> {
    let state = AppState::init().await?;
    tasks::listen_plans(ctx, state.redis.clone()).await
}
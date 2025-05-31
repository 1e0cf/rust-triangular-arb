use crate::app::{AppState, ArbitrageGraph, TokenPair, tasks};
use crate::config::CONFIG;
use anyhow::bail;
use std::sync::Arc;
use tokio::task::{JoinError, JoinSet};
use tokio_util::sync::CancellationToken;
use tracing::info;

pub(crate) async fn run(ctx: CancellationToken) -> anyhow::Result<()> {
    let mut app = Arc::new(AppState::init(CONFIG.redis_url.as_str()).await?);
    println!("123");
    let mut tasks = JoinSet::new();
    let (process_symbol_commands_sender, process_symbol_commands_receiver) = flume::unbounded();
    tasks.spawn(tasks::process_depth_update::worker_pool(
        ctx.clone(),
        5,
        app.clone(),
        process_symbol_commands_receiver,
    ));
    tasks.spawn(tasks::monitor_updates(
        ctx.clone(),
        app.redis.clone(),
        app.local_cache.clone(),
        process_symbol_commands_sender,
    ));

    while let Some(res) = tasks.join_next().await {
        match res {
            Err(e) => {
                ctx.cancel();
                while let Some(_) = tasks.join_next().await {}
                return bail!(e);
            }
            _ => {}
        }
    }
    Ok(())
}

use redis::AsyncCommands;
use ringbuf::traits::Producer;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use utils::{setup_tracing, signal_listener};

mod app;
mod config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();
    let ctx = CancellationToken::new();
    tokio::spawn(signal_listener(ctx.clone()));
    app::run(ctx).await
}

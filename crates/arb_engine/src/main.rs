use crate::config::CONFIG;
use mimalloc::MiMalloc;
use tokio_util::sync::CancellationToken;
use utils::{setup_prometheus, setup_tracing, start_metrics_health_server};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

mod app;
mod config;
mod observability;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();
    let ctx = CancellationToken::new();
    tokio::spawn(start_metrics_health_server(
        ctx.clone(),
        setup_prometheus(),
        CONFIG.metrics_addr.as_str(),
    ));
    // tokio::spawn(signal_listener(ctx.clone()));
    app::run(ctx).await
}

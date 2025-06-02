pub(crate) mod app;
mod config;

use crate::config::CONFIG;
use tokio_util::sync::CancellationToken;
use utils::{setup_prometheus, setup_tracing, start_metrics_health_server};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();
    let ctx = CancellationToken::new();
    tokio::spawn(start_metrics_health_server(
        ctx.clone(),
        setup_prometheus(),
        CONFIG.metrics_addr.as_str(),
    )); // TODO: move signal listener out
    app::run(ctx).await
}

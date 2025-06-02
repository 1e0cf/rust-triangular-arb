use crate::config::CONFIG;
use crate::observability::describe_metrics;
use tokio_util::sync::CancellationToken;
use utils::{setup_prometheus, setup_tracing, start_metrics_health_server};

pub(crate) mod app;
mod config;
mod observability;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();
    describe_metrics();
    let ctx = CancellationToken::new();
    tokio::spawn(start_metrics_health_server(
        ctx.clone(),
        setup_prometheus(),
        CONFIG.metrics_addr.as_str(),
    )); // TODO: move signal listener out
    app::run(ctx).await
}

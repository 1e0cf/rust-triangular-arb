use tokio_util::sync::CancellationToken;
use utils::{setup_prometheus, setup_tracing, start_metrics_health_server};

pub(crate) mod app;
mod config;
mod server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    setup_tracing();
    let ctx = CancellationToken::new();
    tokio::spawn(start_metrics_health_server(
        ctx.clone(),
        setup_prometheus(),
        "0.0.0.0:3000",
    )); // TODO: move signal listener out
    app::run(ctx).await
}

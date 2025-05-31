use crate::signal_listener;
use axum::Router;
use axum::http::StatusCode;
use axum::routing::get;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::future::ready;
use tokio_util::sync::CancellationToken;
use tracing::debug;

pub fn setup_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

pub fn setup_prometheus() -> PrometheusHandle {
    let prometheus_handle = PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install recorder");
    prometheus_handle
}

pub fn metrics_health_app(prometheus_handle: PrometheusHandle) -> Router {
    let app = Router::new()
        .route("/metrics", get(move || ready(prometheus_handle.render())))
        .route("/healthz", get(healthz));
    app
}
pub async fn start_metrics_health_server(
    ctx: CancellationToken,
    prometheus_handle: PrometheusHandle,
    addr: &str,
) -> anyhow::Result<()> {
    let app = metrics_health_app(prometheus_handle);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    debug!("Metrics server listening on {}", listener.local_addr()?);
    axum::serve(listener, app)
        .with_graceful_shutdown(signal_listener(ctx))
        .await?;
    Ok(())
}

async fn healthz() -> StatusCode {
    StatusCode::OK
}

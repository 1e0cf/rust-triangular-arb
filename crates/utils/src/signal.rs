use tokio::select;
use tokio::signal::unix::{SignalKind, signal};
use tokio_util::sync::CancellationToken;
use tracing::info;

pub async fn signal_listener(ctx: CancellationToken) {
    let mut sig_int = signal(SignalKind::interrupt()).expect("sigint stream");
    let mut sig_term = signal(SignalKind::terminate()).expect("sigterm stream");

    select! {
        _ = sig_int.recv() => {},
        _ = sig_term.recv() => {},
    }
    info!("shutdown signal received");
    ctx.cancel();
}

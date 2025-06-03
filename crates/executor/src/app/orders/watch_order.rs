use crate::app::state::BinanceClient;
use crate::config::CONFIG;
use anyhow::bail;
use binance_spot_connector_rust::trade;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::instrument;
use crate::types::{OrderResp, OrderStatus};
use super::serialize_maybe_resp;


#[instrument(skip_all)]
pub async fn watch_order(
    ctx: CancellationToken,
    order: OrderResp,
    binance_client: Arc<BinanceClient>,
) -> anyhow::Result<()> {
    let start_time = Instant::now();
    let total_duration = Duration::from_secs(CONFIG.order_timeout_sec);
    while start_time.elapsed() < total_duration {
        let request = trade::get_order(order.symbol.as_str()).order_id(order.order_id);
        let response = serialize_maybe_resp(binance_client.send(request).await).await?;
        match response.status {
            OrderStatus::Filled => {
                return Ok(());
            }
            OrderStatus::PartiallyFilled | OrderStatus::New | OrderStatus::PendingNew => {
                tokio::time::sleep(Duration::from_millis(200)).await;
                continue;
            }
            _ => {
                bail!("unexpected order status: {:?}", response.status);
            }
        }
    }
    let cancel_req = trade::cancel_order(order.symbol.as_str()).order_id(order.order_id);
    match binance_client.send(cancel_req).await {
        Err(e) => {
            bail!("error while canceling order: {:?}", e);
        }
        _ => {}
    }

    bail!("order {} timed out", order.order_id);
}

use std::sync::Arc;
use anyhow::bail;
use binance_spot_connector_rust::trade;
use binance_spot_connector_rust::trade::order::{Side, TimeInForce};
use metrics::counter;
use tokio_util::sync::CancellationToken;
use tracing::info;
use shared_types::arb_engine;
use super::serialize_maybe_resp;
use crate::app::state::BinanceClient;
use crate::types::OrderResp;

pub async fn execute_order(ctx: CancellationToken, binance_client: Arc<BinanceClient>, order: &arb_engine::v1::Order) -> anyhow::Result<OrderResp> {
    let side = {
        if order.side == "BUY" {
            Side::Buy
        } else {
            Side::Sell
        }
    };
    let request = trade::new_order(order.symbol.as_str(), side, "LIMIT")
        .quantity(order.amount)
        .price(order.price)
        .time_in_force(TimeInForce::Gtc);
    info!("Creating order: {} {} {} at {}", order.side, order.amount,order.symbol, order.price);
    let response = binance_client.send(request).await;
    counter!("executor_orders_sent_total").increment(1);
    match serialize_maybe_resp(response).await {
        Ok(response) => Ok(response),
        Err(e) => {
            counter!("executor_orders_failed_total").increment(1);
            bail!("{:?}", e);
        }
    }
}
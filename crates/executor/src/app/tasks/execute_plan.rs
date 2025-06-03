use crate::app::state::BinanceClient;

use crate::config::CONFIG;
use anyhow::bail;
use binance_spot_connector_rust::http::Credentials;

use binance_spot_connector_rust::hyper::{BinanceHttpClient, Response};
use binance_spot_connector_rust::trade;
use binance_spot_connector_rust::trade::order::{Side, TimeInForce};
use metrics::{counter, histogram};
use serde::Deserialize;
use shared_types::arb_engine;
use std::sync::Arc;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::{info, instrument};
use crate::app::orders::{execute_order, watch_order};
use crate::types::OrderStatus;

#[instrument(skip_all)]
pub async fn execute_plan(
    ctx: CancellationToken,
    plan: arb_engine::v1::ArbitragePlan,
    binance_client: Arc<BinanceClient>,
) -> anyhow::Result<()> {
    info!("Executing plan: {:?}", plan);
    let start = Instant::now();
    for order in &plan.orders {
        let resp = execute_order(ctx.clone(), binance_client.clone(), order).await?;
        if let OrderStatus::Filled = resp.status {
            continue;
        }
        watch_order(ctx.clone(), resp, binance_client.clone()).await?;
    }
    histogram!("executor_cycle_complete_seconds").record(start.elapsed().as_secs_f64());
    counter!("executor_cycles_executed_success_total").increment(1);
    info!("Successfully executed plan");
    Ok(())
}


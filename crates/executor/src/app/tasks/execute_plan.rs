use std::sync::Arc;
use binance_spot_connector_rust::http::Credentials;
use binance_spot_connector_rust::hyper::{BinanceHttpClient, Error};
use binance_spot_connector_rust::trade;
use binance_spot_connector_rust::trade::order::{Side, TimeInForce};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use shared_types::arb_engine;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument};
use crate::config::CONFIG;

#[instrument(skip(ctx, plan))]
pub async fn execute_plan(
    ctx: CancellationToken,
    plan: arb_engine::v1::ArbitragePlan,
) -> anyhow::Result<()> {
    let client = BinanceHttpClient::default().credentials(Credentials::from_hmac(
        CONFIG.api_key.as_str(),
        CONFIG.api_secret.as_str(),
    ));
    info!("executing plan: {:?}", plan);
    for order in plan.orders {
        let side = {
            if order.side == "BUY" {
                Side::Buy
            } else {
                Side::Sell
            }
        };
        info!("quantity: {:?}", order.amount);
        info!("price: {:?}", order.price);
        let request = trade::new_order(order.symbol.as_str(), side, "LIMIT")
            .quantity(order.amount)
            .price(order.price)
            .time_in_force(TimeInForce::Fok);

        let response = client.send(request).await;
        if let Ok(response) = response {
            match response.into_body_str().await {
                Ok(body) => {
                    info!("RESPONSE: {}", body);
                }
                Err(e) => {
                    error!("{:?}", e);
                }
            }
        }
    }
    Ok(())
}

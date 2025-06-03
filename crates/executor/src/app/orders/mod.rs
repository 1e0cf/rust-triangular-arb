use anyhow::bail;
use binance_spot_connector_rust::hyper::Response;
use binance_spot_connector_rust::hyper::Error as BinanceError;
use crate::types::OrderResp;

mod watch_order;
mod execute_order;

pub use watch_order::watch_order;
pub use execute_order::execute_order;

async fn serialize_maybe_resp(
    response: Result<Response, BinanceError>,
) -> anyhow::Result<OrderResp> {
    let resp = match response {
        Ok(resp) => {
            resp
        }
        Err(e) => {
            bail!("{:?}", e);
        }
    };
    let body = match resp.into_body_str().await {
        Ok(body) => body,
        Err(e) => {
            bail!("{:?}", e);
        }
    };
    let order: OrderResp = serde_json::from_str(body.as_str())?;
    Ok(order)
}
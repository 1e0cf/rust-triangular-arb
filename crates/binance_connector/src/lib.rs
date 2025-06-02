pub mod types;

use crate::types::exchange_info::{ExchangeInfoResp, PairInfo};
use anyhow::bail;
use binance_spot_connector_rust::hyper::BinanceHttpClient;
use binance_spot_connector_rust::market;

pub async fn fetch_exchange_info() -> Result<Vec<PairInfo>, anyhow::Error> {
    let client = BinanceHttpClient::default();
    let req = market::exchange_info();
    let resp = match client.send(req).await {
        Ok(resp) => resp,
        Err(e) => {
            bail!("{:?}", e);
        }
    };
    let body: ExchangeInfoResp = match resp.into_body_str().await {
        Ok(body) => serde_json::from_str(body.as_str())?,
        Err(e) => {
            bail!("{:?}", e);
        }
    };
    Ok(body.symbols)
}

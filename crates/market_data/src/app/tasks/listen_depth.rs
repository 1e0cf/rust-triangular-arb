use std::str::FromStr;
use anyhow::bail;
use binance_spot_connector_rust::market_stream::partial_depth_100ms;
use binance_spot_connector_rust::tokio_tungstenite::BinanceWebSocketClient;
use futures_util::{SinkExt, StreamExt};
use metrics::{counter};
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use serde::Deserialize;
use shared_types::market_data;
use std::time::{SystemTime, UNIX_EPOCH};
use rust_decimal::Decimal;
use tokio::select;
use tokio::task::JoinSet;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument, trace};

pub async fn depth_ws_connections_pool(
    ctx: CancellationToken,
    symbols: impl IntoIterator<Item = String>,
    streams_per_connection: usize,
    redis: ConnectionManager,
) -> anyhow::Result<()> {
    let mut tasks = JoinSet::new();
    let mut symbols = symbols.into_iter();
    let mut counter = 0;
    loop {
        let chunk: Vec<String> = symbols.by_ref().take(streams_per_connection).collect();
        if chunk.is_empty() {
            break;
        }
        tasks.spawn(depth_ws_connection(
            ctx.clone(),
            counter,
            chunk,
            redis.clone(),
        ));
        counter += 1;
    }
    while let Some(res) = tasks.join_next().await {
        if let Err(e) = res {
            error!(?e, "worker task panicked");
        }
    }
    Ok(())
}

#[instrument(skip_all, fields(connection_id = id))]
pub async fn depth_ws_connection(
    ctx: CancellationToken,
    id: u32,
    symbols: impl IntoIterator<Item = String>,
    redis: ConnectionManager,
) -> anyhow::Result<()> {
    info!("Starting...");
    let (mut conn, _) = BinanceWebSocketClient::connect_async_default().await?;
    let streams: Vec<_> = symbols
        .into_iter()
        .map(|symbol| partial_depth_100ms(symbol.as_ref(), 20).into())
        .collect();
    conn.subscribe(streams.iter()).await;
    loop {
        select! {
            _ = ctx.cancelled() => {
                info!("Stop the work");
                break;
            },
            maybe_msg = conn.as_mut().next() => {
                match maybe_msg {
                    Some(Ok(msg)) => {
                        match msg {
                            Message::Text(txt) => {
                                if let Ok(message) = serde_json::from_str::<WsDepthMessage>(&txt) {
                                    handle_ws_depth_message(message, redis.clone()).await?;
                                }
                            },
                            Message::Ping(payload) => {
                                conn.as_mut().send(Message::Pong(payload)).await?;
                                info!("Pong");
                            },
                            _ => bail!("Unexpected message type"),
                        }
                    }
                    Some(Err(e)) => {
                        error!("{}", e);
                        bail!(e);
                    }
                    None => {
                        info!("Stream closed");
                    }
                }
            }

        }
    }
    Ok(())
}
async fn handle_ws_depth_message(
    msg: WsDepthMessage,
    mut redis: ConnectionManager,
) -> anyhow::Result<()> {
    // let p = |val: &[String; 2]| -> (Decimal, Decimal) {
    //     (
    //         Decimal::from_str(val[0]).unwrap(),
    //         Decimal::from_str(val[1]).unwrap(),
    //     )
    // };
    // let bid_levels = msg.data.bids.iter().map(p).collect();
    // let ask_levels = msg.data.asks.iter().map(p).collect();
    let bid_levels = msg.data.bids;
    let ask_levels = msg.data.asks;
    let symbol = msg.stream.split('@').next().unwrap().to_uppercase();
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
    let update = market_data::v1::DepthUpdate {
        symbol,
        bid_levels,
        ask_levels,
        timestamp,
    };
    let supdate = serde_json::to_string(&update)?;
    let _: String = redis
        .xadd("market_stream", "*", &[("data", &supdate)])
        .await?;
    trace!("sent update to redis: {:?}", update);
    counter!("depth_updates").increment(1);
    Ok(())
}

#[derive(Debug, Deserialize)]
struct WsDepthMessage {
    stream: String,
    data: WsDepthMessageData,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WsDepthMessageData {
    last_update_id: u64,
    bids: Vec<[Decimal; 2]>,
    asks: Vec<[Decimal; 2]>,
}

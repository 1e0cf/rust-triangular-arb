use anyhow::bail;
use backoff::{Error as BackoffError, ExponentialBackoffBuilder, future::retry};
use binance_spot_connector_rust::market_stream::partial_depth_100ms;
use binance_spot_connector_rust::tokio_tungstenite::{BinanceWebSocketClient, WebSocketState};
use futures_util::{SinkExt, StreamExt};
use metrics::counter;
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use redis::streams::StreamMaxlen;
use rust_decimal::Decimal;
use serde::Deserialize;
use shared_types::market_data;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::TcpStream;
use tokio::select;
use tokio::task::JoinSet;
use tokio_tungstenite::MaybeTlsStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, instrument, trace};
use crate::app::constants::REDIS_STREAM_LEN_APPROX;

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
    info!("Starting connection initialization");
    let backoff = ExponentialBackoffBuilder::new()
        .with_initial_interval(Duration::from_millis(100))
        .with_max_interval(Duration::from_secs(30))
        .with_max_elapsed_time(Some(Duration::from_secs(200))) // keep retrying forever
        .build();
    let streams: Vec<_> = symbols
        .into_iter()
        .map(|symbol| partial_depth_100ms(symbol.as_ref(), 20).into())
        .collect();
    retry(backoff, || async {
        if ctx.is_cancelled() {
            return Ok(());
        }
        let (mut conn, _) = match BinanceWebSocketClient::connect_async_default().await {
            Ok(conn) => conn,
            Err(e) => {
                return Err(BackoffError::permanent(anyhow::anyhow!(
                    "{}",
                    e.to_string()
                )));
            }
        };
        conn.subscribe(streams.iter()).await;
        loop {
            select! {
                _ = ctx.cancelled() => {
                    info!("Context cancelled, stop the work");
                    return Ok(());
                },
                maybe_msg = conn.as_mut().next() => {
                    let err = handle_ws_message(maybe_msg, &mut conn, redis.clone()).await;
                    if let Err(e) = err {
                        error!(?e);
                        return Err(BackoffError::transient(e));
                    }
                }
            }
        }
    })
    .await?;
    Ok(())
}
type MaybeMessage = Option<Result<Message, tokio_tungstenite::tungstenite::Error>>;
type WsConnection = WebSocketState<MaybeTlsStream<TcpStream>>;
async fn handle_ws_message(
    msg: MaybeMessage,
    conn: &mut WsConnection,
    redis: ConnectionManager,
) -> anyhow::Result<()> {
    match msg {
        Some(Ok(msg)) => match msg {
            Message::Text(txt) => {
                if let Ok(message) = serde_json::from_str::<WsDepthMessage>(&txt) {
                    handle_ws_depth_message(message, redis.clone()).await?;
                }
            }
            Message::Ping(payload) => {
                trace!("Received WebSocket ping frame");
                conn.as_mut().send(Message::Pong(payload)).await?;
                trace!("Sent WebSocket pong response");
            }
            _ => bail!("unexpected message type"),
        },
        Some(Err(e)) => {
            error!("{}", e);
            bail!(e);
        }
        None => {
            info!("Stream closed");
        }
    }
    Ok(())
}
#[instrument(name = "ws_depth_handler", skip_all)]
async fn handle_ws_depth_message(
    msg: WsDepthMessage,
    mut redis: ConnectionManager,
) -> anyhow::Result<()> {
    counter!("market_data_depth_updates_total").increment(1);
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
        .xadd_maxlen(
            "market_stream",
            StreamMaxlen::Approx(REDIS_STREAM_LEN_APPROX),
            "*",
            &[("data", &supdate)],
        )
        .await?;
    trace!("sent update to redis: {:?}", update);
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
    // last_update_id: u64,
    bids: Vec<[Decimal; 2]>,
    asks: Vec<[Decimal; 2]>,
}

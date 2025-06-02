use crate::app::{CacheEntry, LocalCache};
use anyhow::bail;
use flume::Sender;
use metrics::counter;
use redis::AsyncCommands;
use redis::aio::{ConnectionManager};
use redis::streams::{StreamReadOptions, StreamReadReply};
use ringbuf::traits::RingBuffer;
use shared_types::market_data;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument};

#[instrument(skip_all)]
pub async fn monitor_updates(
    cancellation_token: CancellationToken,
    mut redis: ConnectionManager,
    local_cache: Arc<LocalCache>,
    process_symbol_commands_sender: Sender<String>,
) -> anyhow::Result<()> {
    let opts = StreamReadOptions::default().count(10).block(1000);
    info!("Started");
    loop {
        select! {
            _ = cancellation_token.cancelled() => {
                info!("Stop the work");
                break;
            }
            reply = redis.xread_options(
                &["market_stream"],
                &["$"],
                &opts,
            ) => {
                let reply: StreamReadReply = reply?;
                for stream_key in reply.keys {
                    for stream_id in stream_key.ids {
                        match parse_market_stream(stream_id.map) {
                            Ok(data) => {
                                counter!("arb_engine_incoming_depth_updates_total").increment(1);
                                let symbol = data.symbol.clone();
                                debug!("Writing to cache: {}-{}", symbol, data.timestamp);
                                let mut cache = local_cache.entry(data.symbol.clone()).or_insert(CacheEntry::default());
                                cache.buf.push_overwrite(data);

                                if !cache.in_flight.swap(true, Ordering::SeqCst) {
                                    debug!("Sending process symbol command: {}", symbol);
                                    _ = process_symbol_commands_sender.send_async(symbol).await?;
                                }
                            }
                            Err(e) => {
                                error!("deserializing error: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn parse_market_stream(
    mut data: HashMap<String, redis::Value>,
) -> anyhow::Result<market_data::v1::DepthUpdate> {
    if let Some(redis::Value::BulkString(json_raw)) = data.remove("data") {
        let json_str = String::from_utf8(json_raw)?;
        let depth_update: market_data::v1::DepthUpdate = serde_json::from_str(json_str.as_str())?;
        return Ok(depth_update);
    }
    bail!("invalid message type")
}

use crate::app::state::BinanceClient;
use crate::app::tasks;
use anyhow::bail;
use metrics::{counter, histogram};
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use redis::streams::{StreamReadOptions, StreamReadReply};
use shared_types::arb_engine;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::select;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

pub async fn listen_plans(
    ctx: CancellationToken,
    mut redis: ConnectionManager,
    binance_client: Arc<BinanceClient>,
) -> anyhow::Result<()> {
    let opts = StreamReadOptions::default().count(10).block(1000);
    info!("Plans listener started");
    loop {
        select! {
            _ = ctx.cancelled() => {
                info!("Stop the work");
                break;
            }
            reply = redis.xread_options(
                &["plan_stream"],
                &["$"],
                &opts,
            ) => {
                let reply: StreamReadReply = reply?;
                for stream_key in reply.keys {
                    for stream_id in stream_key.ids {
                        match parse_plan_stream(stream_id.map) {
                            Ok(plan) => {
                                counter!("executor_incoming_plans_total").increment(1);
                                let ctx_inner = ctx.clone();
                                let binance_client_inner = binance_client.clone();
                                tokio::spawn(async move {
                                    if let Err(e) = tasks::execute_plan(ctx_inner, plan, binance_client_inner).await {
                                        counter!("executor_cycles_executed_fail_total").increment(1);
                                        error!(?e);
                                    }
                                }).await?;
                            }
                            Err(e) => {
                                eprintln!("Ошибка десериализации: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn parse_plan_stream(
    mut data: HashMap<String, redis::Value>,
) -> anyhow::Result<arb_engine::v1::ArbitragePlan> {
    if let Some(redis::Value::BulkString(json_raw)) = data.remove("data") {
        let json_str = String::from_utf8(json_raw)?;
        let plan: arb_engine::v1::ArbitragePlan = serde_json::from_str(json_str.as_str())?;
        return Ok(plan);
    }
    bail!("invalid message type!")
}

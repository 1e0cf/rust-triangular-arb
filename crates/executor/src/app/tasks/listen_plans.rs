use crate::app::tasks;
use anyhow::bail;
use metrics::counter;
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use redis::streams::{StreamReadOptions, StreamReadReply};
use shared_types::{arb_engine};
use std::collections::HashMap;
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

pub async fn listen_plans(
    ctx: CancellationToken,
    mut redis: ConnectionManager,
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
                            Ok(data) => {
                                counter!("executor_incoming_plans_total").increment(1);
                                let ctx_inner = ctx.clone();
                                tokio::spawn(async move {
                                    match tasks::execute_plan(ctx_inner, data).await {
                                        Ok(_) => {
                                            counter!("executor_cycles_executed_success_total").increment(1);
                                        },
                                        Err(e) => {
                                            counter!("executor_cycles_executed_fail_total").increment(1);
                                            error!(?e);
                                        }
                                    }
                                });
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

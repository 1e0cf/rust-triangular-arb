use std::collections::HashMap;
use anyhow::bail;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use redis::streams::{StreamReadOptions, StreamReadReply};
use tokio::select;
use tokio_util::sync::CancellationToken;
use tracing::info;
use shared_types::{arb_engine, market_data};
use crate::app::tasks;

pub async fn listen_plans(ctx: CancellationToken, mut redis: ConnectionManager) -> anyhow::Result<()> {
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
                                // tokio::spawn(tasks::execute_plan(ctx.clone(), data));
                                tasks::execute_plan(ctx.clone(), data).await?;
                                ctx.cancel();
                                break;
                                return Ok(());
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

fn parse_plan_stream(mut data: HashMap<String, redis::Value>) -> anyhow::Result<arb_engine::v1::ArbitragePlan> {
    if let Some(redis::Value::BulkString(json_raw)) = data.remove("data") {
    let json_str = String::from_utf8(json_raw)?;
    let plan: arb_engine::v1::ArbitragePlan = serde_json::from_str(json_str.as_str())?;
    return Ok(plan);
}
    bail!("invalid message type!")
}
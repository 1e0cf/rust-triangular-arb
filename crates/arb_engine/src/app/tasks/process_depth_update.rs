use crate::app::constants::REDIS_STREAM_LEN_APPROX;
use crate::app::math::calculate_plan;
use crate::app::{AppState, Triangle};
use crate::config::CONFIG;
use anyhow::bail;
use flume::Receiver;
use metrics::counter;
use redis::AsyncCommands;
use redis::streams::StreamMaxlen;
use ringbuf::traits::{Consumer, Producer};
use rust_decimal::Decimal;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::select;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, instrument};

pub async fn worker_pool(
    ctx: CancellationToken,
    workers: u32,
    state: Arc<AppState>,
    process_symbol_commands_receiver: Receiver<String>,
) -> anyhow::Result<()> {
    let mut tasks = JoinSet::new();

    for i in 0..workers {
        tasks.spawn(process_symbol_worker(
            ctx.clone(),
            i,
            state.clone(),
            process_symbol_commands_receiver.clone(),
        ));
    }

    while let Some(res) = tasks.join_next().await {
        if let Err(e) = res {
            error!(?e, "worker task panicked");
        }
    }
    Ok(())
}

// Listen channel and proceed local cache HashMap key
#[instrument(skip_all, fields(worker_id = id))]
async fn process_symbol_worker(
    ctx: CancellationToken,
    id: u32,
    state: Arc<AppState>,
    process_symbol_commands_receiver: Receiver<String>,
) -> anyhow::Result<()> {
    info!("start the work");
    loop {
        select! {
            _ = ctx.cancelled() => {
                info!("stop the work");
                break
            },
            command = process_symbol_commands_receiver.recv_async() => {
                match command {
                    Ok(symbol) => {
                        process_one_symbol(symbol, state.clone()).await?;
                    },
                    Err(e) => {
                        bail!("failed while receive depth update: {:?}", e);
                    }
                }
            }
        }
    }
    Ok(())
}

// Proceed local cache HashMap key
async fn process_one_symbol(symbol: String, state: Arc<AppState>) -> anyhow::Result<()> {
    let mut local_symbols = Vec::new();
    let triangles: Vec<&Triangle> = state
        .graph
        .get_triangles_by_symbol(symbol.as_str())
        .into_iter()
        .filter(|triangle| {
            for pair in &triangle.pairs {
                if !local_symbols.contains(&pair.symbol) {
                    if !state.local_cache.contains_key(&pair.symbol.to_string()) {
                        return false;
                    }
                    local_symbols.push(pair.symbol.clone());
                }
            }
            true
        })
        .collect();
    for triangle in triangles {
        let prices = get_price_vol(triangle, state.clone()).expect("failed to get prices");
        let plan = calculate_plan(
            triangle,
            prices.clone(),
            CONFIG.cycle_volume,
            CONFIG.fee_percent,
        );

        if let Ok(plan) = plan {
            counter!("arb_engine_plans_total").increment(1);
            debug!("{:?}", plan);
            if plan.profit_percent >= CONFIG.min_profit_perc {
                info!("{:?}", plan);
                let splan = serde_json::to_string(&plan)?;
                let mut redis = state.redis.clone();
                let _: String = redis
                    .xadd_maxlen(
                        "plan_stream",
                        StreamMaxlen::Approx(REDIS_STREAM_LEN_APPROX),
                        "*",
                        &[("data", &splan)],
                    )
                    .await?;
                counter!("arb_engine_outgoing_plans_total").increment(1);
            }
        } else if let Err(e) = plan {
            counter!("arb_engine_failed_plans_total").increment(1);
            error!("{}", e.to_string());
        }
    }
    let entry = state.local_cache.get_mut(symbol.as_str()).unwrap();
    entry.in_flight.store(false, Ordering::SeqCst);

    Ok(())
}

fn get_price_vol(triangle: &Triangle, state: Arc<AppState>) -> Option<[Vec<[Decimal; 2]>; 3]> {
    let symbol_1 = triangle.pairs[0].symbol.to_string();
    let symbol_2 = triangle.pairs[1].symbol.to_string();
    let symbol_3 = triangle.pairs[2].symbol.to_string();

    let orders_1 = {
        let mut entry = state.local_cache.get_mut(&symbol_1)?;
        let depth = entry.buf.try_pop()?;
        let asks = depth.ask_levels.clone();
        _ = entry.buf.try_push(depth);
        asks
    };
    let orders_2 = {
        let mut entry = state.local_cache.get_mut(&symbol_2)?;
        let depth = entry.buf.try_pop()?;
        let orders;
        if triangle.intermediate_token == triangle.pairs[0].base
            && triangle.intermediate_token == triangle.pairs[1].base
        {
            orders = depth.bid_levels.clone();
        } else {
            orders = depth.ask_levels.clone();
        }
        _ = entry.buf.try_push(depth);
        orders
    };
    let orders_3 = {
        let mut entry = state.local_cache.get_mut(&symbol_3)?;
        let depth = entry.buf.try_pop()?;
        let bids = depth.bid_levels.clone();
        _ = entry.buf.try_push(depth);
        bids
    };
    Some([orders_1, orders_2, orders_3])
}

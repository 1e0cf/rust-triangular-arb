mod arbitrage_graph;
mod math;
mod runner;
pub(crate) mod tasks;

pub use arbitrage_graph::{ArbitrageGraph, TokenPair, Triangle};
use binance_connector::fetch_exchange_info;
use dashmap::DashMap;
use redis::AsyncCommands;
use redis::aio::ConnectionManager;
use ringbuf::StaticRb;
pub(crate) use runner::run;
use shared_types::market_data;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use crate::config::CONFIG;

pub(crate) type LocalCache = DashMap<String, CacheEntry>;

#[derive(Default)]
pub(crate) struct CacheEntry {
    pub buf: StaticRb<market_data::v1::DepthUpdate, 5>,
    pub in_flight: AtomicBool,
}

pub struct AppState {
    pub redis: ConnectionManager,
    // pub pair_infos: Vec<PairInfo>,
    pub local_cache: Arc<LocalCache>,
    pub graph: Arc<ArbitrageGraph>,
}

impl AppState {
    pub async fn init(redis_url: &str) -> anyhow::Result<Self> {
        let r = redis::Client::open(redis_url)?;
        let _: String = r.get_multiplexed_async_connection().await?.ping().await?; // Check if redis available
        let redis = ConnectionManager::new(r).await?;
        let graph = Arc::new(Self::init_graph().await?);
        Ok(Self {
            redis,
            local_cache: Arc::new(DashMap::new()),
            graph,
        })
    }
    async fn init_graph() -> anyhow::Result<ArbitrageGraph> {
        let pairs_info = fetch_exchange_info().await?;
        let mut graph = ArbitrageGraph::new(CONFIG.stablecoins.iter().map(|c| c.as_str()).collect());
        pairs_info.into_iter().for_each(|pair| {
            graph.add_token_pair(TokenPair::new(pair));
        });
        graph.build_triangles();
        // for (i, triangle) in graph.triangles.iter().take(20).enumerate() {
        //     info!("{}. {} -> {} -> {}", i, triangle.stablecoin, triangle.intermediate_token, triangle.target_token);
        // }
        // for (i, triangle) in graph.triangles.iter().take(20).enumerate() {
        //     info!("{}. {} -> {} -> {}", i, triangle.pairs[0].quote, triangle.pairs[0].base, triangle.pairs[1].symbol);
        // }
        Ok(graph)
    }
}

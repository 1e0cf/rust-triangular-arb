mod arbitrage_graph;
mod runner;
pub(crate) mod tasks;
mod math;

use anyhow::bail;
pub use arbitrage_graph::{ArbitrageGraph, TokenPair, Triangle};
use binance_connector::fetch_exchange_info;
use dashmap::DashMap;
use ringbuf::StaticRb;
pub(crate) use runner::run;
use serde::Deserialize;
use shared_types::market_data;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use redis::aio::ConnectionManager;
use redis::{AsyncCommands};

pub(crate) type LocalCache = DashMap<String, CacheEntry>;

#[derive(Default)]
pub(crate) struct CacheEntry {
    pub buf: StaticRb<market_data::v1::DepthUpdate, 5>,
    pub in_flight: AtomicBool,
}

pub struct AppState {
    pub redis: ConnectionManager,
    symbols: HashMap<String, String>,
    pub local_cache: Arc<LocalCache>,
    pub graph: Arc<ArbitrageGraph>,
}

impl AppState {
    pub async fn init(redis_url: &str) -> anyhow::Result<Self> {
        let mut r = redis::Client::open(redis_url)?;
        let _: String = r.get_multiplexed_async_connection().await?.ping().await?; // Check if redis available
        let redis = ConnectionManager::new(r).await?;
        let graph = Arc::new(Self::init_graph().await?);
        Ok(Self {
            redis,
            symbols: HashMap::new(),
            local_cache: Arc::new(DashMap::new()),
            graph,
        })
    }
    async fn init_graph() -> anyhow::Result<ArbitrageGraph> {
        let pairs_info = fetch_exchange_info().await?;
        let mut graph = ArbitrageGraph::new(vec!["USDT"]);
        pairs_info.into_iter().for_each(|pair| {
            graph.add_token_pair(
                TokenPair::new(pair.base_asset.as_str(), pair.quote_asset.as_str())
            );
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

#[derive(Debug, Deserialize)]
struct ExchangeInfoResp {
    pub symbols: Vec<Symbol>,
}
impl ExchangeInfoResp {
    fn to_pairs_vec(&self) -> Vec<TokenPair> {
        self.symbols
            .iter()
            .map(|s| TokenPair::new(&s.base_asset, &s.quote_asset))
            .collect()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Symbol {
    pub base_asset: String,
    pub quote_asset: String,
}

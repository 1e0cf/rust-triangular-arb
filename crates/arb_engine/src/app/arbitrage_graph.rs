use ahash::{AHashMap, AHashSet};
use binance_connector::types::exchange_info::PairInfo;
use petgraph::graph::{DiGraph, NodeIndex};
use std::sync::Arc;

pub struct ArbitrageGraph {
    graph: DiGraph<Arc<str>, TokenPair>,
    token_to_node: AHashMap<Arc<str>, NodeIndex>,

    stablecoins: AHashSet<Arc<str>>,
    regular_tokens: AHashSet<Arc<str>>,

    pub triangles: Vec<Triangle>,

    // indexes
    triangles_by_symbol: AHashMap<Arc<str>, Vec<usize>>,
    triangles_by_stablecoin: AHashMap<Arc<str>, Vec<usize>>,
    triangles_by_intermediate: AHashMap<Arc<str>, Vec<usize>>,
    triangles_by_target: AHashMap<Arc<str>, Vec<usize>>,

    // Cache for fastest search
    pairs_from_stable: AHashMap<Arc<str>, Vec<Arc<str>>>,
    pairs_to_stable: AHashMap<Arc<str>, Vec<Arc<str>>>,
}
impl ArbitrageGraph {
    pub fn new(stablecoins: Vec<&str>) -> Self {
        Self {
            graph: DiGraph::new(),
            token_to_node: AHashMap::new(),
            stablecoins: stablecoins.iter().map(|&s| Arc::from(s)).collect(),
            regular_tokens: AHashSet::new(),
            triangles: Vec::new(),
            triangles_by_symbol: AHashMap::new(),
            triangles_by_stablecoin: AHashMap::new(),
            triangles_by_intermediate: AHashMap::new(),
            triangles_by_target: AHashMap::new(),
            pairs_from_stable: AHashMap::new(),
            pairs_to_stable: AHashMap::new(),
        }
    }
    pub fn ensure_token(&mut self, token: &str) -> NodeIndex {
        let token_arc = Arc::from(token);

        if let Some(&node_idx) = self.token_to_node.get(&token_arc) {
            return node_idx;
        }

        let node_idx = self.graph.add_node(token_arc.clone());
        self.token_to_node.insert(token_arc.clone(), node_idx);

        // Классифицируем токен
        if self.stablecoins.contains(&token_arc) {
            // Уже в стейблкоинах
        } else {
            self.regular_tokens.insert(token_arc);
        }

        node_idx
    }
    pub fn add_token_pair(&mut self, pair: TokenPair) {
        let base_idx = self.ensure_token(&pair.base);
        let quote_idx = self.ensure_token(&pair.quote);

        self.graph.add_edge(base_idx, quote_idx, pair.clone());
        self.graph.add_edge(quote_idx, base_idx, pair.clone());

        self.update_stablecoin_caches(&pair);
    }

    pub fn build_triangles(&mut self) {
        self.triangles.clear();
        self.triangles_by_stablecoin.clear();
        self.triangles_by_intermediate.clear();
        self.triangles_by_target.clear();

        for stablecoin in &self.stablecoins {
            if let Some(tokens_from_stable) = self.pairs_from_stable.get(stablecoin) {
                for i in 0..tokens_from_stable.len() {
                    for j in (i + 1)..tokens_from_stable.len() {
                        let token_b = &tokens_from_stable[i];
                        let token_c = &tokens_from_stable[j];

                        if let Some(triangle) =
                            self.try_build_stable_triangle(stablecoin, token_b, token_c)
                        {
                            self.triangles.push(triangle);
                        }
                        // if let Some(triangle) =
                        //     self.try_build_stable_triangle(stablecoin, token_c, token_b)
                        // {
                        //     self.triangles.push(triangle);
                        // }
                    }
                }
            }
        }

        self.build_triangle_indices();
    }

    fn update_stablecoin_caches(&mut self, pair: &TokenPair) {
        // If base - stablecoin. Need to test USDC/USDT pairs
        // if self.stablecoins.contains(&pair.base) {
        //     self.pairs_from_stable
        //         .entry(pair.base.clone())
        //         .or_insert_with(Vec::new)
        //         .push(pair.quote.clone());
        //
        //     self.pairs_to_stable
        //         .entry(pair.quote.clone())
        //         .or_insert_with(Vec::new)
        //         .push(pair.base.clone());
        // }

        // If quote - stablecoin (default)
        if self.stablecoins.contains(&pair.quote) {
            self.pairs_from_stable
                .entry(pair.quote.clone())
                .or_insert_with(Vec::new)
                .push(pair.base.clone());

            self.pairs_to_stable
                .entry(pair.base.clone())
                .or_insert_with(Vec::new)
                .push(pair.quote.clone());
        }
    }
    fn try_build_stable_triangle(
        &self,
        stable: &Arc<str>,
        token_b: &Arc<str>,
        token_c: &Arc<str>,
    ) -> Option<Triangle> {
        // Triangle: stable -> token_b -> token_c -> stable
        if let (Some(&stable_node), Some(&node_b), Some(&node_c)) = (
            self.token_to_node.get(stable),
            self.token_to_node.get(token_b),
            self.token_to_node.get(token_c),
        ) {
            // Проверяем существование всех рёбер
            let edge_stable_to_b = self.graph.find_edge(stable_node, node_b);
            let edge_b_to_c = self.graph.find_edge(node_b, node_c);
            let edge_c_to_stable = self.graph.find_edge(node_c, stable_node);

            if let (Some(sb), Some(bc), Some(cs)) =
                (edge_stable_to_b, edge_b_to_c, edge_c_to_stable)
            {
                let pair_sb = self.graph[sb].clone();
                let pair_bc = self.graph[bc].clone();
                let pair_cs = self.graph[cs].clone();

                return Some(Triangle::new(
                    stable.clone(),
                    token_b.clone(),
                    token_c.clone(),
                    pair_sb,
                    pair_bc,
                    pair_cs,
                ));
            }
        }
        None
    }

    fn build_triangle_indices(&mut self) {
        for (idx, triangle) in self.triangles.iter().enumerate() {
            // Индекс по стейблкоину
            self.triangles_by_stablecoin
                .entry(triangle.stablecoin.clone())
                .or_insert_with(Vec::new)
                .push(idx);

            // Индекс по промежуточному токену
            self.triangles_by_intermediate
                .entry(triangle.intermediate_token.clone())
                .or_insert_with(Vec::new)
                .push(idx);

            // Индекс по целевому токену
            self.triangles_by_target
                .entry(triangle.target_token.clone())
                .or_insert_with(Vec::new)
                .push(idx);
            for pair in &triangle.pairs {
                // loop unrolling
                self.triangles_by_symbol
                    .entry(pair.symbol.clone())
                    .or_insert_with(Vec::new)
                    .push(idx);
            }
        }
    }
    /*pub fn get_triangles_by_token(&self, token: &str) -> Vec<&Triangle> {
        let token_arc = Arc::from(token);
        let mut result = Vec::new();

        // Поиск как промежуточного токена
        if let Some(indices) = self.triangles_by_intermediate.get(&token_arc) {
            for &idx in indices {
                result.push(&self.triangles[idx]);
            }
        }

        // Поиск как целевого токена
        if let Some(indices) = self.triangles_by_target.get(&token_arc) {
            for &idx in indices {
                result.push(&self.triangles[idx]);
            }
        }

        result
    }*/
    pub fn get_triangles_by_symbol(&self, symbol: &str) -> Vec<&Triangle> {
        // TODO: Option or zero Vec?
        let mut result = Vec::new();
        if let Some(indices) = self.triangles_by_symbol.get(&Arc::from(symbol)) {
            for &idx in indices {
                result.push(&self.triangles[idx]);
            }
        }
        result
    }
}

#[derive(Debug, Clone)]
pub struct Triangle {
    pub stablecoin: Arc<str>,         // A
    pub intermediate_token: Arc<str>, // B
    pub target_token: Arc<str>,       // C
    pub pairs: [TokenPair; 3],        // A/B, B/C, C/A
}

impl Triangle {
    pub fn new(
        stablecoin: Arc<str>,
        intermediate: Arc<str>,
        target: Arc<str>,
        pair_ab: TokenPair,
        pair_bc: TokenPair,
        pair_ca: TokenPair,
    ) -> Self {
        Self {
            stablecoin,
            intermediate_token: intermediate,
            target_token: target,
            pairs: [pair_ab, pair_bc, pair_ca],
        }
    }
}

#[derive(Debug, Clone)]
pub struct TokenPair {
    pub base: Arc<str>,
    pub quote: Arc<str>,
    pub symbol: Arc<str>,
    pub info: PairInfo,
}
impl TokenPair {
    pub fn new(pair_info: PairInfo) -> Self {
        Self {
            base: Arc::from(pair_info.base_asset.clone()),
            quote: Arc::from(pair_info.quote_asset.clone()),
            symbol: Arc::from(pair_info.symbol.clone()),
            info: pair_info,
        }
    }
}

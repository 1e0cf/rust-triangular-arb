pub mod v1 {
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};

    //
    #[derive(Debug, Serialize, Deserialize)]
    pub struct DepthUpdate {
        pub symbol: String,
        pub bid_levels: Vec<[Decimal; 2]>,
        pub ask_levels: Vec<[Decimal; 2]>,
        pub timestamp: u64,
    }
}

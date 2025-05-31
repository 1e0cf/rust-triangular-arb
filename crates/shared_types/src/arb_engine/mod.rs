pub mod v1 {
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
    pub struct ArbitragePlan {
        pub orders: [Order; 3],
        pub profit_percent: Decimal,
        pub timestamp: u64,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Order {
        pub symbol: String,
        pub price: Decimal,
        pub amount: Decimal,
        pub side: String,
    }
}

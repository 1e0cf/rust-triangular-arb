// /api/v3/exchangeInfo
pub mod exchange_info {
    use rust_decimal::Decimal;
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub(crate) struct ExchangeInfoResp {
        pub symbols: Vec<PairInfo>,
    }

    #[derive(Clone, Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PairInfo {
        pub symbol: String,
        pub base_asset: String,
        pub quote_asset: String,
        pub filters: Vec<Filter>,
    }

    #[derive(Clone, Debug, Deserialize)]
    #[serde(tag = "filterType", rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum Filter {
        #[serde(rename_all = "camelCase")]
        PriceFilter {
            min_price: Decimal,
            max_price: Decimal,
            tick_size: Decimal,
        },
        #[serde(rename_all = "camelCase")]
        LotSize {
            min_qty: Decimal,
            max_qty: Decimal,
            step_size: Decimal,
        },
        #[serde(rename_all = "camelCase")]
        IcebergParts { limit: u32 },
        #[serde(rename_all = "camelCase")]
        MarketLotSize {
            min_qty: Decimal,
            max_qty: Decimal,
            step_size: Decimal,
        },
        #[serde(rename_all = "camelCase")]
        TrailingDelta {
            min_trailing_above_delta: u32,
            max_trailing_above_delta: u32,
            min_trailing_below_delta: u32,
            max_trailing_below_delta: u32,
        },
        #[serde(rename_all = "camelCase")]
        PercentPriceBySide {
            bid_multiplier_up: Decimal,
            bid_multiplier_down: Decimal,
            ask_multiplier_up: Decimal,
            ask_multiplier_down: Decimal,
            avg_price_mins: u32,
        },
        #[serde(rename_all = "camelCase")]
        Notional {
            min_notional: Decimal,
            apply_min_to_market: bool,
            max_notional: Decimal,
            apply_max_to_market: bool,
            avg_price_mins: u32,
        },
        #[serde(rename_all = "camelCase")]
        MaxNumOrders { max_num_orders: u32 },
        #[serde(rename_all = "camelCase")]
        MaxNumAlgoOrders { max_num_algo_orders: u32 },
        #[serde(rename_all = "camelCase")]
        MaxPosition { max_position: Decimal },
    }
}

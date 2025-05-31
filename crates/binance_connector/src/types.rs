use serde::Deserialize;

// /api/v3/exchangeInfo
pub mod exchange_info {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PairInfo {
        pub symbol: String,
        pub base_asset: String,
        pub quote_asset: String,
    }

    #[derive(Debug, Deserialize)]
    pub(crate) struct ExchangeInfoResp {
        pub symbols: Vec<PairInfo>,
    }
}

use metrics::describe_counter;

pub fn describe_metrics() {
    describe_counter!(
        "market_data_depth_updates_total",
        "Count of depth updates from Binance"
    );
}

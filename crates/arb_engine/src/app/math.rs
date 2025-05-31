use crate::app::Triangle;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use rust_decimal_macros::dec;
use shared_types::arb_engine;
use std::time::{SystemTime, UNIX_EPOCH};


const STEP_SIZE: Decimal = dec!(0.0001);

pub fn calculate_plan(
    triangle: &Triangle,
    prices: [Vec<[Decimal; 2]>; 3],
    amount: Decimal,
    fee: Decimal,
) -> arb_engine::v1::ArbitragePlan {
    let amountA = amount;
    let price_1 = prices[0][0][0];
    let price_2 = prices[1][0][0];
    let price_3 = prices[2][0][0];
    let (middle_side, b_is_base) = {
        if triangle.intermediate_token == triangle.pairs[0].base
            && triangle.intermediate_token == triangle.pairs[1].base
        {
            ("SELL".to_string(), true)
        } else {
            ("BUY".to_string(), false)
        }
    };
    let (amountB, amountC) = calculate_amounts(amountA, price_1, price_2, fee, b_is_base);
    let profit_percent = calculate_profit(amountA, amountC, price_3, fee);
    let order_1 = arb_engine::v1::Order {
        symbol: triangle.pairs[0].symbol.to_string(),
        price: price_1,
        amount: amountB,
        side: "BUY".to_string(),
    };
    let order_2_amount = { if b_is_base { amountB } else { amountC } };
    let order_2 = arb_engine::v1::Order {
        symbol: triangle.pairs[1].symbol.to_string(),
        price: price_2,
        amount: order_2_amount,
        side: middle_side,
    };
    let order_3 = arb_engine::v1::Order {
        symbol: triangle.pairs[2].symbol.to_string(),
        price: price_3,
        amount: amountC,
        side: "SELL".to_string(),
    };
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64; // unwrap safe
    arb_engine::v1::ArbitragePlan {
        orders: [order_1, order_2, order_3],
        profit_percent,
        timestamp,
    }
}

fn calculate_amounts(
    amountA: Decimal,
    price_1: Decimal,
    price_2: Decimal,
    fee: Decimal,
    b_is_base: bool,
) -> (Decimal, Decimal) {
    let amountB = ((amountA / price_1 * (dec!(1.0) - fee)).round_dp(8) / STEP_SIZE).floor() * STEP_SIZE;
    // token B can be both base and quote
    let amountC;
    if b_is_base {
        amountC = (amountB * price_2 * (dec!(1.0) - fee)).round_dp(8); // SELL
    } else {
        amountC = (amountB / price_2 * (dec!(1.0) - fee)).round_dp(8); // BUY
    }
    let amountC = (amountC / STEP_SIZE).floor() * STEP_SIZE;
    (amountB, amountC)
}

fn calculate_profit(amountA: Decimal, amountC: Decimal, price_3: Decimal, fee: Decimal) -> Decimal {
    let final_amount = amountC * price_3 * (dec!(1.0) - fee);
    (final_amount - amountA) / amountA * dec!(100.0)
}

#[cfg(test)]
mod arb_math_tests {
    use super::*;
    use crate::app::TokenPair;
    use std::sync::Arc;

    #[test]
    fn test_calculate_plan_buy_middle() {
        let triangle = Triangle::new(
            Arc::from("USDT"),
            Arc::from("BTC"),
            Arc::from("AAVE"),
            TokenPair::new("BTC", "USDT"),
            TokenPair::new("AAVE", "BTC"),
            TokenPair::new("AAVE", "USDT"),
        );
        // let prices = [vec![(100_000.0, 100.0)], vec![(0.0024, 100_000.0)], vec![(250.0, 1_000_000.0)]];
        let prices = [
            vec![[dec!(103831.31), dec!(100.0)]],
            vec![[dec!(0.002408), dec!(100_000.0)]],
            vec![[dec!(249.93), dec!(1_000_000.0)]],
        ];
        let plan = calculate_plan(&triangle, prices, dec!(1000.0), dec!(0.001));
        println!("{:?}", plan);
    }
    #[test]
    fn test_calculate_plan_sell_middle() {
        let triangle = Triangle::new(
            Arc::from("USDT"),
            Arc::from("AAVE"),
            Arc::from("BTC"),
            TokenPair::new("AAVE", "USDT"),
            TokenPair::new("AAVE", "BTC"),
            TokenPair::new("BTC", "USDT"),
        );
        // let prices = [vec![(250.0, 100.0)], vec![(0.0024, 100_000.0)], vec![(100_000.0, 1_000_000.0)]];
        let prices = [
            vec![[dec!(249.93), dec!(100.0)]],
            vec![[dec!(0.002408), dec!(100_000.0)]],
            vec![[dec!(103831.31), dec!(1_000_000.0)]],
        ];
        let plan = calculate_plan(&triangle, prices, dec!(1000.0), dec!(0.001));
        println!("{:?}", plan);
    }
    #[test]
    fn test_calculate_profit() {
        let amountA = dec!(1000.0); // USDT
        let amountC = dec!(1100.0); // TTK
        let price_3 = dec!(1.0);
        let fee = dec!(0.001);
        assert_eq!(
            calculate_profit(amountA, amountC, price_3, fee),
            dec!(9.89000)
        )
    }
}

#![allow(non_snake_case)]

use crate::app::{TokenPair, Triangle};
use anyhow::bail;
use binance_connector::types::exchange_info::Filter;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use shared_types::arb_engine;
use std::time::{SystemTime, UNIX_EPOCH};
use utils::decimal::DecimalExt;

pub fn calculate_plan(
    triangle: &Triangle,
    prices: [Vec<[Decimal; 2]>; 3],
    amount: Decimal,
    fee: Decimal,
) -> anyhow::Result<arb_engine::v1::ArbitragePlan> {
    let prices = (prices[0][0][0], prices[1][0][0], prices[2][0][0]);

    let step_dp = |i| {
        get_step_size(&triangle.pairs[i])
            .unwrap()
            .normalize()
            .scale()
    };

    let step_dps = (step_dp(0), step_dp(1), step_dp(2));
    let (middle_side, b_is_base) = {
        if triangle.intermediate_token == triangle.pairs[0].base
            && triangle.intermediate_token == triangle.pairs[1].base
        {
            ("SELL".to_string(), true)
        } else {
            ("BUY".to_string(), false)
        }
    };
    let (orders_amounts, _dusts, final_amount) =
        calculate_triangle_amounts(amount, prices, step_dps, fee, b_is_base);
    let profit_percent = calculate_profit_percent(amount, final_amount);
    let not1 = get_notional(&triangle.pairs[0]).unwrap();
    let not2 = get_notional(&triangle.pairs[1]).unwrap();
    let not3 = get_notional(&triangle.pairs[2]).unwrap();

    let order_1 = construct_order(
        triangle.pairs[0].symbol.to_string(),
        prices.0,
        orders_amounts.0,
        "BUY".to_string(),
        not1,
    )?;
    let order_2 = construct_order(
        triangle.pairs[1].symbol.to_string(),
        prices.1,
        orders_amounts.1,
        middle_side,
        not2,
    )?;
    let order_3 = construct_order(
        triangle.pairs[2].symbol.to_string(),
        prices.2,
        orders_amounts.2,
        "SELL".to_string(),
        not3,
    )?;
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis() as u64;
    Ok(arb_engine::v1::ArbitragePlan {
        orders: [order_1, order_2, order_3],
        profit_percent,
        timestamp,
    })
}

fn calculate_triangle_amounts(
    amount: Decimal,
    prices: (Decimal, Decimal, Decimal),
    step_dps: (u32, u32, u32),
    fee_percent: Decimal,
    B_is_base: bool,
) -> (
    (Decimal, Decimal, Decimal),
    (Decimal, Decimal, Decimal),
    Decimal,
) {
    let (order1_amount, order2_amount, order3_amount);
    // let (order1_fee, order2_fee, order3_fee);
    let (tokenB_amount, tokenC_amount, final_amount);
    let (tokenA_dust, tokenB_dust, tokenC_dust);

    (order1_amount, tokenA_dust) =
        calculate_buy_order_amounts(amount, prices.0, step_dps.0, fee_percent); // order 1
    tokenB_amount = order1_amount;
    if B_is_base {
        // SELL
        (order2_amount, tokenC_amount, tokenB_dust) =
            calculate_sell_order_amounts(tokenB_amount, prices.1, step_dps.1, fee_percent);
    } else {
        // BUY
        (order2_amount, tokenB_dust) =
            calculate_buy_order_amounts(tokenB_amount, prices.1, step_dps.1, fee_percent);
        tokenC_amount = order2_amount
    }
    (order3_amount, final_amount, tokenC_dust) =
        calculate_sell_order_amounts(tokenC_amount, prices.2, step_dps.2, fee_percent);
    (
        (order1_amount, order2_amount, order3_amount),
        (tokenA_dust, tokenB_dust, tokenC_dust),
        final_amount,
    )
}

// We know how much quote tokens amount we want to pay, but all orders in base token
fn calculate_buy_order_amounts(
    quote_amount: Decimal,
    price: Decimal,
    step_dp: u32,
    fee_percent: Decimal,
) -> (Decimal, Decimal) {
    let base_amount = quote_amount / price * (Decimal::ONE - fee_percent);
    let order_amount = base_amount.truncate_to_dp(step_dp); // Round to avoid getting a LOT_SIZE error
    let quote_dust = (base_amount - order_amount).round_dp(8);
    (order_amount, quote_dust)
}
// We want to sell all base amount, and avoid LOT_SIZE error
fn calculate_sell_order_amounts(
    base_amount: Decimal,
    price: Decimal,
    step_dp: u32,
    fee_percent: Decimal,
) -> (Decimal, Decimal, Decimal) {
    let order_amount = base_amount.truncate_to_dp(step_dp);
    let quote_amount = (order_amount * price * (dec!(1.0) - fee_percent)).round_dp(8);

    let base_dust = base_amount - order_amount;
    (order_amount, quote_amount, base_dust)
}
fn calculate_profit_percent(amount: Decimal, final_amount: Decimal) -> Decimal {
    (final_amount - amount) / amount * dec!(100.0)
}

// Get decimal points of step size for base token in pair
fn get_step_size(pair: &TokenPair) -> Option<Decimal> {
    for filter in &pair.info.filters {
        match filter {
            Filter::LotSize { step_size, .. } => {
                return Some(step_size.clone());
            }
            _ => {}
        }
    }
    None
}
fn get_notional(pair: &TokenPair) -> Option<Decimal> {
    for filter in &pair.info.filters {
        match filter {
            Filter::Notional { min_notional, .. } => return Some(min_notional.clone()),
            _ => {}
        }
    }
    None
}
fn construct_order(
    symbol: String,
    price: Decimal,
    amount: Decimal,
    side: String,
    min_notional: Decimal,
) -> anyhow::Result<arb_engine::v1::Order> {
    let volume = amount * price;
    if volume < min_notional {
        bail!(
            "Volume for pair {} is lower than minNotional: {} < {}",
            symbol,
            volume,
            min_notional
        )
    }
    Ok(arb_engine::v1::Order {
        symbol,
        price,
        amount,
        side,
    })
}

#[cfg(test)]
mod arb_math_tests {
    use super::*;
    use crate::app::TokenPair;
    use binance_connector::types::exchange_info::PairInfo;
    use std::sync::Arc;

    #[test]
    fn test_calculate_buy_order_amounts() {
        let usdt_amount = dec!(500);
        let btc_price = dec!(105_000);
        let step_points = 5; // 0.00001
        let fee = dec!(0.001);
        let (order_amount, usdt_dust) =
            calculate_buy_order_amounts(usdt_amount, btc_price, step_points, fee);
        assert_eq!(order_amount, dec!(0.00476));
        assert_eq!(usdt_dust, dec!(0.2))
    }

    #[test]
    fn test_calculate_plan_buy_middle() {
        let info1 = PairInfo {
            symbol: "AAVEUSDT".to_string(),
            base_asset: "AAVE".to_string(),
            quote_asset: "USDT".to_string(),
            filters: vec![
                Filter::LotSize {
                    min_qty: dec!(0.00100000),
                    max_qty: dec!(900000.00000000),
                    step_size: dec!(0.00100000),
                },
                Filter::Notional {
                    min_notional: dec!(5.00000000),
                    apply_min_to_market: true,
                    max_notional: dec!(9000000.00000000),
                    apply_max_to_market: false,
                    avg_price_mins: 5,
                },
            ],
        };
        let info2 = PairInfo {
            symbol: "AAVEBTC".to_string(),
            base_asset: "AAVE".to_string(),
            quote_asset: "BTC".to_string(),
            filters: vec![
                Filter::LotSize {
                    min_qty: dec!(0.00100000),
                    step_size: dec!(0.00100000),
                    max_qty: dec!(92141578.00000000),
                },
                Filter::Notional {
                    min_notional: dec!(0.00010000),
                    apply_min_to_market: true,
                    max_notional: dec!(9000000.00000000),
                    apply_max_to_market: false,
                    avg_price_mins: 5,
                },
            ],
        };
        let info3 = PairInfo {
            symbol: "BTCUSDT".to_string(),
            base_asset: "BTC".to_string(),
            quote_asset: "USDT".to_string(),
            filters: vec![
                Filter::LotSize {
                    min_qty: dec!(00001000),
                    max_qty: dec!(9000.00000000),
                    step_size: dec!(0.00001000),
                },
                Filter::Notional {
                    min_notional: dec!(5.00000000),
                    apply_min_to_market: true,
                    max_notional: dec!(9000000.00000000),
                    apply_max_to_market: false,
                    avg_price_mins: 5,
                },
            ],
        };
        let triangle = Triangle::new(
            Arc::from("USDT"),
            Arc::from("AAVE"),
            Arc::from("BTC"),
            TokenPair::new(info1),
            TokenPair::new(info2),
            TokenPair::new(info3),
        );
        let prices = [
            vec![[dec!(240), dec!(200)]],
            vec![[dec!(0.0024), dec!(2000)]],
            vec![[dec!(105_000), dec!(100)]],
        ];
        let fee = dec!(0.001);
        let amount = dec!(1000);
        let plan = calculate_plan(&triangle, prices, amount, fee);
        assert!(plan.is_ok());
        println!("{:?}", plan.unwrap());
    }
    //     #[test]
    //     fn test_calculate_plan_sell_middle() {
    //         let triangle = Triangle::new(
    //             Arc::from("USDT"),
    //             Arc::from("AAVE"),
    //             Arc::from("BTC"),
    //             TokenPair::new("AAVE", "USDT"),
    //             TokenPair::new("AAVE", "BTC"),
    //             TokenPair::new("BTC", "USDT"),
    //         );
    //         // let prices = [vec![(250.0, 100.0)], vec![(0.0024, 100_000.0)], vec![(100_000.0, 1_000_000.0)]];
    //         let prices = [
    //             vec![[dec!(249.93), dec!(100.0)]],
    //             vec![[dec!(0.002408), dec!(100_000.0)]],
    //             vec![[dec!(103831.31), dec!(1_000_000.0)]],
    //         ];
    //         let plan = calculate_plan(&triangle, prices, dec!(1000.0), dec!(0.001));
    //         println!("{:?}", plan);
    //     }
    //     #[test]
    //     fn test_calculate_profit() {
    //         let amountA = dec!(1000.0); // USDT
    //         let amountC = dec!(1100.0); // TTK
    //         let price_3 = dec!(1.0);
    //         let fee = dec!(0.001);
    //         assert_eq!(
    //             calculate_profit(amountA, amountC, price_3, fee),
    //             dec!(9.89000)
    //         )
    //     }
}

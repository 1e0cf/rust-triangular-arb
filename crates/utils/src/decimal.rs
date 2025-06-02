use rust_decimal::Decimal;

pub trait DecimalExt {
    fn truncate_to_dp(&self, dp: u32) -> Decimal;
}
impl DecimalExt for Decimal {
    fn truncate_to_dp(&self, dp: u32) -> Decimal {
        let scale_factor = Decimal::new(10i64.pow(dp), 0);
        let result = (*self * scale_factor).trunc() / scale_factor;
        result
    }
}

#[cfg(test)]
mod decimal_ext_tests {
    use super::DecimalExt;
    use rust_decimal_macros::dec;

    #[test]
    fn test_truncate_to_dp_down() {
        let val = dec!(0.12345678).truncate_to_dp(5);
        assert_eq!(val + dec!(0.00000001), dec!(0.12345001));
    }
    #[test]
    fn test_truncate_to_dp_up() {
        let val = dec!(0.12345678).truncate_to_dp(10);
        assert_eq!(val + dec!(0.0000000001), dec!(0.1234567801));
    }
}

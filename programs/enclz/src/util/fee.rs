use anchor_lang::prelude::*;

use crate::constants::PROTOCOL_FEE_BPS;
use crate::errors::EnclzError;

const BPS_DENOMINATOR: u64 = 10_000;

/// Returns `(total, fee)` where `total = amount + ceil(amount * BPS / 10_000)`
/// and `fee = ceil(amount * BPS / 10_000)`.
/// The fee is computed with integer ceil: `(amount * bps + BPS_DENOMINATOR - 1) / BPS_DENOMINATOR`.
/// The recipient receives exactly `amount`; the fee is added on top.
pub fn compute_fee(amount: u64) -> Result<(u64, u64)> {
    let fee = (amount as u128)
        .checked_mul(PROTOCOL_FEE_BPS as u128)
        .ok_or(EnclzError::InvalidAmount)?
        .checked_add(BPS_DENOMINATOR as u128 - 1)
        .ok_or(EnclzError::InvalidAmount)?
        .checked_div(BPS_DENOMINATOR as u128)
        .ok_or(EnclzError::InvalidAmount)?;
    let fee = u64::try_from(fee).map_err(|_| EnclzError::InvalidAmount)?;
    let total = amount.checked_add(fee).ok_or(EnclzError::InvalidAmount)?;
    Ok((total, fee))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_usdc_fee_is_ten_basis_points() {
        let (total, fee) = compute_fee(1_000_000).unwrap();
        assert_eq!(fee, 1_000);
        assert_eq!(total, 1_001_000);
    }

    #[test]
    fn standard_x402_amount_matches_example() {
        // From issue #33: amount=300_000 should give fee=300, total=300_300
        let (total, fee) = compute_fee(300_000).unwrap();
        assert_eq!(fee, 300);
        assert_eq!(total, 300_300);
    }

    #[test]
    fn zero_amount_yields_zero_fee_and_zero_total() {
        let (total, fee) = compute_fee(0).unwrap();
        assert_eq!(fee, 0);
        assert_eq!(total, 0);
    }

    #[test]
    fn tiny_amount_rounds_up_to_one_unit_fee() {
        // Previously this truncated to zero; with ceil, minimum fee is 1.
        let (total, fee) = compute_fee(99).unwrap();
        assert_eq!(fee, 1);
        assert_eq!(total, 100);
    }

    #[test]
    fn total_minus_fee_always_equals_amount() {
        for &amount in &[1u64, 100, 1_000, 1_000_000, 5_000_000, u64::MAX / 2] {
            let (total, fee) = compute_fee(amount).unwrap();
            assert_eq!(total.checked_sub(fee), Some(amount));
        }
    }

    #[test]
    fn near_max_safe_amount_succeeds() {
        // Additive math overflows when amount + ceil(amount * 10 / 10000) > u64::MAX.
        // u64::MAX * 10000 / 10010 ≈ 18428379664409152590 is the largest safe amount.
        // Test a large but safe value well below that bound.
        let safe_max = u64::MAX / 2 + 1_000_000;
        let (total, fee) = compute_fee(safe_max).unwrap();
        assert_eq!(total.checked_sub(fee), Some(safe_max));
        assert!(total > safe_max);
    }

    #[test]
    fn minimum_unit_transfer_incurs_fee() {
        let (total, fee) = compute_fee(1).unwrap();
        assert_eq!(fee, 1);
        assert_eq!(total, 2);
    }
}

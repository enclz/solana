use anchor_lang::prelude::*;

use crate::constants::PROTOCOL_FEE_BPS;
use crate::errors::EnclzError;

const BPS_DENOMINATOR: u64 = 10_000;

pub fn compute_fee(amount: u64) -> Result<(u64, u64)> {
    let fee = (amount as u128)
        .checked_mul(PROTOCOL_FEE_BPS as u128)
        .ok_or(EnclzError::InvalidAmount)?
        .checked_div(BPS_DENOMINATOR as u128)
        .ok_or(EnclzError::InvalidAmount)?;
    let fee = u64::try_from(fee).map_err(|_| EnclzError::InvalidAmount)?;
    let net = amount.checked_sub(fee).ok_or(EnclzError::InvalidAmount)?;
    Ok((net, fee))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_usdc_fee_is_ten_basis_points() {
        let (net, fee) = compute_fee(1_000_000).unwrap();
        assert_eq!(fee, 1_000);
        assert_eq!(net, 999_000);
    }

    #[test]
    fn zero_amount_yields_zero_fee_and_zero_net() {
        let (net, fee) = compute_fee(0).unwrap();
        assert_eq!(fee, 0);
        assert_eq!(net, 0);
    }

    #[test]
    fn tiny_amount_truncates_fee_to_zero_in_favor_of_agent() {
        let (net, fee) = compute_fee(99).unwrap();
        assert_eq!(fee, 0);
        assert_eq!(net, 99);
    }

    #[test]
    fn fee_plus_net_always_equals_amount() {
        for &amount in &[1u64, 100, 1_000, 1_000_000, 5_000_000, u64::MAX / 2] {
            let (net, fee) = compute_fee(amount).unwrap();
            assert_eq!(net.checked_add(fee), Some(amount));
        }
    }

    #[test]
    fn max_u64_does_not_overflow() {
        let (net, fee) = compute_fee(u64::MAX).unwrap();
        assert_eq!(net.checked_add(fee), Some(u64::MAX));
    }
}

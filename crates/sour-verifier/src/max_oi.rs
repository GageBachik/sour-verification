//! v0.6.0 max_oi notional-micros enforcement model.
//!
//! Mirrors the on-chain check at
//! `programs/sour/src/instructions/upsert_position.rs` (post v0.6.0):
//!
//! ```text
//!   side_oi       = max(new_long_oi, new_short_oi)        // qmax_storage units
//!   side_notional = side_oi × mark / FIXED_ONE             // µUSDC
//!   require!(side_notional <= max_oi_notional_micros, OverMaxOi);
//! ```
//!
//! Decoupled from Solana runtime so Kani can decide it without dragging in
//! the program-error type.

/// Q32.32 fixed-point scale shared with `Position.qmax` storage and
/// `Market.long_oi`/`short_oi`. Same value as
/// `programs/sour-math/src/curve.rs::FIXED_ONE`.
pub const FIXED_ONE: u128 = 1u128 << 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaxOiError {
    /// Post-trade gross-side notional exceeds `max_oi_notional_micros`.
    /// Maps to on-chain `SourError::OverMaxOi` (Custom:19).
    OverMaxOi,
    /// `side_oi × mark` overflowed u128.
    Overflow,
}

/// Compute the post-trade gross-side notional in µUSDC and compare to the cap.
/// Returns `Ok(())` if the open admits, `Err(OverMaxOi)` if the cap rejects.
#[inline]
pub fn check_max_oi_notional(
    new_long_oi: u64,
    new_short_oi: u64,
    mark_micros: u128,
    max_oi_notional_micros: u64,
) -> Result<(), MaxOiError> {
    let long = new_long_oi as u128;
    let short = new_short_oi as u128;
    let side: u128 = if long > short { long } else { short };
    let product = side
        .checked_mul(mark_micros)
        .ok_or(MaxOiError::Overflow)?;
    let side_notional_u128 = product / FIXED_ONE;
    let side_notional: u64 = if side_notional_u128 > u64::MAX as u128 {
        u64::MAX
    } else {
        side_notional_u128 as u64
    };
    if side_notional <= max_oi_notional_micros {
        Ok(())
    } else {
        Err(MaxOiError::OverMaxOi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn under_cap_admits() {
        // qmax_underlying = 80, mark = $1.40 µUSDC = 1_400_000.
        // qmax_storage = 80 × FIXED_ONE = 343_597_383_680.
        // side_notional = 80 × 1_400_000 = 112_000_000 µUSDC = $112.
        let qmax_storage = 80u64 * (FIXED_ONE as u64);
        let mark = 1_400_000u128;
        let cap = 200_000_000u64; // $200 cap
        assert_eq!(
            check_max_oi_notional(qmax_storage, 0, mark, cap),
            Ok(()),
            "$112 notional should admit under $200 cap"
        );
    }

    #[test]
    fn over_cap_rejects() {
        // Same setup, cap tightened to $100.
        let qmax_storage = 80u64 * (FIXED_ONE as u64);
        let mark = 1_400_000u128;
        let cap = 100_000_000u64; // $100 cap
        assert_eq!(
            check_max_oi_notional(qmax_storage, 0, mark, cap),
            Err(MaxOiError::OverMaxOi),
            "$112 notional should reject above $100 cap"
        );
    }

    #[test]
    fn cross_market_parity_btc_vs_xrp() {
        // BTC: $80,000 mark, qmax_underlying = 0.001 → notional = $80.
        let btc_mark = 80_000_000_000u128;
        let btc_qmax = (80_000_000u128 * FIXED_ONE) / btc_mark;
        // XRP: $1.40 mark, qmax_underlying = 57.14 → notional = $80.
        let xrp_mark = 1_400_000u128;
        let xrp_qmax = (80_000_000u128 * FIXED_ONE) / xrp_mark;
        let cap = 100_000_000u64; // $100 cap, same dollars on both
        assert_eq!(
            check_max_oi_notional(btc_qmax as u64, 0, btc_mark, cap),
            Ok(()),
            "BTC $80 notional under $100 cap"
        );
        assert_eq!(
            check_max_oi_notional(xrp_qmax as u64, 0, xrp_mark, cap),
            Ok(()),
            "XRP $80 notional under $100 cap"
        );
    }

    #[test]
    fn uses_max_side_not_sum() {
        // long_oi = 50 (notional $70), short_oi = 50 (notional $70) — sum $140 > cap $100,
        // but max-side $70 <= cap $100 → admits.
        let mark = 1_400_000u128;
        let qmax_one_side = 50u64 * (FIXED_ONE as u64);
        let cap = 100_000_000u64;
        assert_eq!(
            check_max_oi_notional(qmax_one_side, qmax_one_side, mark, cap),
            Ok(()),
            "balanced 50/50 uses max-side notional $70 under cap $100"
        );
    }
}

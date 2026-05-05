//! v0.5.1-P1c aggregate-budget enforcement model.
//!
//! Mirrors three on-chain surfaces from `programs/sour/src/state.rs` +
//! `programs/sour/src/instructions/upsert_position.rs` +
//! `programs/sour/src/instructions/recompute_aggregate.rs` so they can be
//! reasoned about by Kani without dragging in Solana runtime types:
//!
//!   - `Protocol::update_aggregate(delta) -> Result<(), AggregateError>`
//!     (signed-add with overflow + underflow guards),
//!   - `Market::worst_case_lp_loss(max_mm_bps, mark_micros) -> u128`
//!     (per-market exposure in µUSDC),
//!   - `aggregate_cap(total_assets, aggregate_budget_bps, max_mm_bps) -> u128`
//!     and the upsert-enforcement predicate that combines them.
//!
//! SPEC NOTE — formula deviation from the locked Part 4 spec:
//! the spec writes `perp_side = |long_oi - short_oi| × max_mm_bps / 10_000`,
//! which assumes OI is denominated in µUSDC. In the codebase
//! `Market.long_oi`/`short_oi` are stored in `qmax_storage` units
//! (FIXED_ONE-scaled base units; see v0.4 B1 storage scaling). To produce
//! a result in µUSDC — required so it sums dimensionally with `curve_side`
//! (already µUSDC per `curve_max_lp_loss_per_position_micros`) — the
//! implementation multiplies by `mark_micros` and divides by FIXED_ONE.
//!
//! The proofs in `kani_harnesses.rs` operate on this implementation, so
//! they certify the as-built code rather than the as-written spec.

use solana_program_error::ProgramError;

/// Q32.32 fixed-point scale shared with `Position.qmax` storage and
/// `Market.long_oi`/`short_oi`. Match `state.rs` constant.
pub const FIXED_ONE: u128 = 1u128 << 32;

/// Aggregate-budget error tags. Distinct enum (rather than `ProgramError`)
/// so Kani harnesses can pattern-match without pulling Solana runtime
/// internals into the proof goal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateError {
    /// `Protocol::update_aggregate` was passed a negative delta whose
    /// absolute value exceeds the current counter — would push it negative.
    /// Maps to on-chain `SourError::AggregateBudgetUnderflow` (Custom:43).
    Underflow,
    /// `update_aggregate` overflow on the i128 add (impossibly rare; defensive).
    /// Maps to on-chain `SourError::Overflow`.
    Overflow,
}

impl From<AggregateError> for ProgramError {
    fn from(_: AggregateError) -> Self {
        ProgramError::ArithmeticOverflow
    }
}

/// Protocol fields touched by the aggregate-budget hot path.
///
/// Field semantics match `state.rs::Protocol` byte-for-byte for the slots
/// used here. Other Protocol fields (admin, paused, etc.) are irrelevant
/// to the cap arithmetic and omitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolModel {
    /// Hot-path counter — running sum of every market's
    /// `worst_case_lp_loss(max_mm_bps, mark)`.
    pub aggregate_max_lp_loss: u128,
    /// Cap numerator scaling factor. Default 10_000 (100%).
    /// 0 = treat as default 10_000 in the on-chain handler; the proof
    /// model assumes the caller already resolved that fallback.
    pub aggregate_budget_bps: u16,
    /// Max maintenance-margin bps across all tiers (LETHAL = 100).
    /// Cap formula divisor.
    pub max_mm_bps: u16,
}

impl ProtocolModel {
    /// Mirror `Protocol::update_aggregate` from `state.rs`:
    ///
    /// ```text
    ///   prior      = self.aggregate_max_lp_loss
    ///   new_signed = (prior as i128) checked_add delta  → Overflow if None
    ///   if new_signed < 0 → Underflow
    ///   else self.aggregate_max_lp_loss = new_signed as u128
    /// ```
    ///
    /// Returns `()` on success; `AggregateError` on overflow or underflow.
    /// Saturating? No — strictly checked so cascade drift is observable.
    #[inline]
    pub fn update_aggregate(&mut self, delta: i128) -> Result<(), AggregateError> {
        let prior_i128 = self.aggregate_max_lp_loss as i128;
        // Defensive: even u128::MAX as i128 is i128::MIN-ish on truncation,
        // but our cap predicate ensures `aggregate_max_lp_loss <= cap`, and
        // cap <= total_assets × 10_000 / 1 = total_assets × 10_000, which
        // for any sane vault fits comfortably in i128 positive range.
        if prior_i128 < 0 {
            return Err(AggregateError::Overflow);
        }
        let new_signed = prior_i128
            .checked_add(delta)
            .ok_or(AggregateError::Overflow)?;
        if new_signed < 0 {
            return Err(AggregateError::Underflow);
        }
        self.aggregate_max_lp_loss = new_signed as u128;
        Ok(())
    }
}

/// Per-market state needed by `worst_case_lp_loss`. Mirrors the relevant
/// `Market` fields from `state.rs::Market`. Other Market fields (oracle
/// addresses, fees, etc.) are irrelevant to the per-market exposure
/// arithmetic and omitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarketModel {
    /// Open interest aggregates in `qmax_storage` units (FIXED_ONE-scaled).
    pub long_oi: u64,
    pub short_oi: u64,
    /// CURVE-mode aggregate caps already in µUSDC (computed by
    /// `curve_max_lp_loss_per_position_micros`).
    pub curve_max_lp_loss_long: u128,
    pub curve_max_lp_loss_short: u128,
    /// On-chain reference price (`Market.price_smoothed`). Used by
    /// `recompute_aggregate` directly; `upsert_position` passes `pi_open`
    /// (the just-read oracle mark).
    pub price_smoothed: u128,
}

impl MarketModel {
    /// Byte-exact mirror of `Market::worst_case_lp_loss` from `state.rs:547`:
    ///
    /// ```text
    ///   net_skew   = |long_oi − short_oi|              (qmax_storage units)
    ///   perp_side  = net_skew × mark × max_mm_bps / 10_000 / FIXED_ONE   (µUSDC)
    ///   curve_side = max(curve_long, curve_short)                        (µUSDC)
    ///   result     = perp_side saturating_add curve_side
    /// ```
    ///
    /// Uses `saturating_mul` on the perp side to keep a single-market wrap
    /// from blowing up the cascade — matches the on-chain implementation.
    #[inline]
    pub fn worst_case_lp_loss(&self, max_mm_bps: u16, mark_micros: u128) -> u128 {
        let long_oi = self.long_oi as u128;
        let short_oi = self.short_oi as u128;
        let net_skew: u128 = if long_oi > short_oi {
            long_oi - short_oi
        } else {
            short_oi - long_oi
        };
        let perp_side = net_skew
            .saturating_mul(mark_micros)
            .saturating_mul(max_mm_bps as u128)
            / 10_000u128
            / FIXED_ONE;
        let curve_side = if self.curve_max_lp_loss_long > self.curve_max_lp_loss_short {
            self.curve_max_lp_loss_long
        } else {
            self.curve_max_lp_loss_short
        };
        perp_side.saturating_add(curve_side)
    }
}

/// Aggregate cap formula from `upsert_position.rs:579`:
///
/// ```text
///   cap = total_assets × aggregate_budget_bps / max(max_mm_bps, 1)
/// ```
///
/// `max_mm_bps == 0` is treated as `1` to mirror the `.max(1)` guard that
/// keeps the divisor from zeroing. (The on-chain handler also normalizes 0
/// to a default of 100 earlier; that fallback is the caller's responsibility
/// in this model.)
#[inline]
pub fn aggregate_cap(total_assets: u64, aggregate_budget_bps: u16, max_mm_bps: u16) -> u128 {
    let lp_nav = total_assets as u128;
    let bps = aggregate_budget_bps as u128;
    let denom = (max_mm_bps as u128).max(1);
    lp_nav.saturating_mul(bps) / denom
}

/// Predicate matching the on-chain enforcement at `upsert_position.rs:648`:
///
/// ```text
///   require!(
///       proposed_aggregate >= 0
///       && (proposed_aggregate as u128) <= aggregate_cap,
///       AggregateBudgetExceeded
///   );
/// ```
///
/// `proposed_aggregate = (protocol.aggregate_max_lp_loss as i128) + delta`.
#[inline]
pub fn upsert_check_passes(protocol: &ProtocolModel, delta: i128, total_assets: u64) -> bool {
    let proposed = match (protocol.aggregate_max_lp_loss as i128).checked_add(delta) {
        Some(v) => v,
        None => return false,
    };
    if proposed < 0 {
        return false;
    }
    let cap = aggregate_cap(
        total_assets,
        protocol.aggregate_budget_bps,
        protocol.max_mm_bps,
    );
    (proposed as u128) <= cap
}

/// Sum `Σ market.worst_case_lp_loss(max_mm_bps, market.price_smoothed)` across
/// the supplied markets, matching the body of
/// `recompute_aggregate.rs::handler` (modulo the runtime-side PDA
/// re-derivation + program-owner check, which authenticates that the
/// supplied accounts ARE markets — the math itself is just this sum).
///
/// Returns `Err(Overflow)` on `checked_add` overflow, matching the on-chain
/// `SourError::Overflow` path.
#[inline]
pub fn recompute_aggregate(markets: &[MarketModel], max_mm_bps: u16) -> Result<u128, AggregateError> {
    let mut sum: u128 = 0;
    for m in markets {
        let term = m.worst_case_lp_loss(max_mm_bps, m.price_smoothed);
        sum = sum.checked_add(term).ok_or(AggregateError::Overflow)?;
    }
    Ok(sum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worst_case_zero_when_balanced_no_curve() {
        let m = MarketModel {
            long_oi: 1_000_000,
            short_oi: 1_000_000,
            curve_max_lp_loss_long: 0,
            curve_max_lp_loss_short: 0,
            price_smoothed: 100_000_000,
        };
        assert_eq!(m.worst_case_lp_loss(100, 100_000_000), 0);
    }

    #[test]
    fn worst_case_uses_max_curve_side() {
        let m = MarketModel {
            long_oi: 0,
            short_oi: 0,
            curve_max_lp_loss_long: 7_000_000,
            curve_max_lp_loss_short: 3_000_000,
            price_smoothed: 100_000_000,
        };
        assert_eq!(m.worst_case_lp_loss(100, 100_000_000), 7_000_000);
    }

    #[test]
    fn update_aggregate_round_trip_zero_delta() {
        let mut p = ProtocolModel {
            aggregate_max_lp_loss: 5_000_000,
            aggregate_budget_bps: 1_000,
            max_mm_bps: 100,
        };
        p.update_aggregate(0).unwrap();
        assert_eq!(p.aggregate_max_lp_loss, 5_000_000);
    }

    #[test]
    fn update_aggregate_underflow_rejects() {
        let mut p = ProtocolModel {
            aggregate_max_lp_loss: 100,
            aggregate_budget_bps: 1_000,
            max_mm_bps: 100,
        };
        assert_eq!(p.update_aggregate(-101), Err(AggregateError::Underflow));
        // counter unchanged on Err
        assert_eq!(p.aggregate_max_lp_loss, 100);
    }

    #[test]
    fn cap_formula_matches_default_devnet_config() {
        // Devnet default: total_assets=$106 in micros, budget=10_000 (100%), max_mm=100.
        // Cap = 106_000_000 × 10_000 / 100 = 10_600_000_000 µUSDC = $10_600.
        assert_eq!(aggregate_cap(106_000_000, 10_000, 100), 10_600_000_000);
    }

    #[test]
    fn cap_with_zero_max_mm_uses_one() {
        // .max(1) guard so we don't divide by zero.
        assert_eq!(aggregate_cap(100, 10_000, 0), 1_000_000);
    }

    #[test]
    fn recompute_matches_per_market_sum() {
        let m1 = MarketModel {
            long_oi: 0,
            short_oi: 0,
            curve_max_lp_loss_long: 1_000_000,
            curve_max_lp_loss_short: 0,
            price_smoothed: 50_000_000,
        };
        let m2 = MarketModel {
            long_oi: 0,
            short_oi: 0,
            curve_max_lp_loss_long: 0,
            curve_max_lp_loss_short: 2_500_000,
            price_smoothed: 80_000_000,
        };
        let sum = recompute_aggregate(&[m1, m2], 100).unwrap();
        assert_eq!(sum, 3_500_000);
    }

    #[test]
    fn upsert_check_passes_at_exact_cap() {
        let p = ProtocolModel {
            aggregate_max_lp_loss: 0,
            aggregate_budget_bps: 10_000,
            max_mm_bps: 100,
        };
        // cap = 100 × 10_000 / 100 = 10_000. Delta exactly 10_000 should PASS.
        assert!(upsert_check_passes(&p, 10_000, 100));
        assert!(!upsert_check_passes(&p, 10_001, 100));
    }
}

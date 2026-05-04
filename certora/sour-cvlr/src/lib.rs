use cvlr::prelude::*;
use sour_math::{curve, epsilon, fees, margin};

// ---------------------------------------------------------------------------
// v0.5.0 LP-scaled per-user cap rule (compile-only mirror of Kani Invariant 1)
//
// Mirrors proof_per_user_cap_bounds_single_position_loss.
// No Certora rule run executed (template-only stack); compile smoke verifies
// the arithmetic is well-typed and matches sour-math signatures.
// ---------------------------------------------------------------------------
#[rule]
pub fn rule_per_user_cap_bounds_single_loss() {
    let lp_nav: u64 = nondet();
    let risk_budget_bps: u16 = nondet();
    let max_mm_bps: u16 = nondet();
    let mm_bps: u16 = nondet();
    let notional: u64 = nondet();

    cvlr_assume!(risk_budget_bps >= 1 && risk_budget_bps <= 10_000);
    cvlr_assume!(max_mm_bps >= 1 && max_mm_bps <= 1_000);
    cvlr_assume!(mm_bps <= max_mm_bps);

    let cap_u128 = (lp_nav as u128)
        .saturating_mul(risk_budget_bps as u128)
        / (max_mm_bps as u128);
    let cap: u64 = if cap_u128 > u64::MAX as u128 {
        u64::MAX
    } else {
        cap_u128 as u64
    };

    cvlr_assume!(notional <= cap);

    let worst_loss = (notional as u128).saturating_mul(mm_bps as u128) / 10_000;
    let budget = (lp_nav as u128).saturating_mul(risk_budget_bps as u128) / 10_000;

    cvlr_assert!(worst_loss <= budget);
}

#[rule]
pub fn rule_fee_never_exceeds_notional() {
    let notional: u64 = nondet();
    let fee_micros: u32 = nondet();
    cvlr_assume!(fee_micros <= 1_000_000);

    let fee = fees::fee_for_notional(notional, fee_micros).unwrap();
    cvlr_assert!(fee <= notional);
}

#[rule]
pub fn rule_epsilon_is_capped() {
    let override_bps: u32 = nondet();
    let dynamic_scale: u32 = nondet();
    let allocated_lp_bps: u16 = nondet();
    let total_assets: u64 = nondet();
    let max_epsilon_bps: u32 = nondet();
    cvlr_assume!(allocated_lp_bps <= 10_000);

    let eps = epsilon::effective_epsilon_bps(
        override_bps,
        dynamic_scale,
        allocated_lp_bps,
        total_assets,
        max_epsilon_bps,
    )
    .unwrap();
    cvlr_assert!(eps <= max_epsilon_bps);
}

#[rule]
pub fn rule_curve_fill_ratios_are_bounded() {
    let p_lo: u64 = nondet();
    let width: u64 = nondet();
    let pi: u64 = nondet();
    cvlr_assume!(p_lo <= 1_000_000_000_000);
    cvlr_assume!(width > 0);
    cvlr_assume!(width <= 1_000_000_000_000);

    let p_hi = p_lo + width;
    let long = curve::long_fill_ratio(p_lo, p_hi, pi as u128).unwrap();
    let short = curve::short_fill_ratio(p_lo, p_hi, pi as u128).unwrap();

    cvlr_assert!(long >= 0);
    cvlr_assert!(long <= curve::FIXED_ONE);
    cvlr_assert!(short >= 0);
    cvlr_assert!(short <= curve::FIXED_ONE);
}

#[rule]
pub fn rule_expected_failure_curve_lp_loss_counts_fractional_capacity() {
    let qmax_storage: u64 = nondet();
    let entry: u64 = nondet();
    let p_lo: u64 = nondet();
    let p_hi: u64 = nondet();
    let is_short: bool = nondet();

    cvlr_assume!(qmax_storage > 0);
    cvlr_assume!((qmax_storage as u128) < (curve::FIXED_ONE as u128));
    cvlr_assume!(p_lo < entry);
    cvlr_assume!(entry < p_hi);
    cvlr_assume!(p_hi - p_lo <= 1_000_000_000);

    let current = margin::curve_max_lp_loss_per_position_micros(
        qmax_storage,
        entry,
        p_lo,
        p_hi,
        is_short,
    )
    .unwrap();
    let adverse = if is_short {
        (entry - p_lo) as u128
    } else {
        (p_hi - entry) as u128
    };
    let precise = ((qmax_storage as u128) * adverse) / (curve::FIXED_ONE as u128);

    cvlr_assert!(current == precise);
}


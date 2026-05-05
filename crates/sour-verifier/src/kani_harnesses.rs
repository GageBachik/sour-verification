use crate::accounting::{self, VaultModel};
use sour_math::{curve, epsilon, fees, margin};

// ---------------------------------------------------------------------------
// Invariant 1 — LP-scaled per-user cap bounds single-position worst loss
//
// Formula (v0.5.0 LP-scaled cap):
//   per_user_cap   = lp_nav × risk_budget_bps / max_mm_bps
//   worst_LP_loss  = notional × mm_bps / 10_000
//
// Theorem: notional ≤ per_user_cap  ⟹  worst_LP_loss ≤ lp_nav × risk_budget_bps / 10_000
//
// CBMC-decidable proof strategy:
// ─────────────────────────────
// CBMC times out on symbolic multiplication above u8 × u8 (bitvector explosion).
// This is the same class of limitation that blocks `proof_fee_for_notional_never_exceeds_notional_under_100_percent`.
//
// Proof split:
//   Part A (this Kani harness) — numerator bound, u8-only inputs, NO division:
//     If n * max_mm ≤ nav * rbps  AND  mm ≤ max_mm,
//     THEN n * mm ≤ nav * rbps.
//   All inputs are u8; products fit u16. CBMC decides this instantly.
//
//   Part B (Lean `single_position_bound`) — full division form over unbounded Nat,
//   using Nat.div_mul_le_self + Nat.div_le_div_right. General, no bit-width limit.
//
// u8 × u8 products fit u16: 255 × 255 = 65025 < 2^16.
// ---------------------------------------------------------------------------
#[cfg(kani)]
#[kani::proof]
fn proof_per_user_cap_bounds_single_position_loss() {
    // All symbolic variables are u8 so that u8 * u8 products fit u16.
    // This covers the ratio structure: mm/max_mm * max_mm/rbps ≤ 1.
    let nav: u8 = kani::any();     // scaled lp_nav (unit-invariant)
    let rbps: u8 = kani::any_where(|&b: &u8| b >= 1);   // risk_budget_bps ≥ 1
    let max_mm: u8 = kani::any_where(|&b: &u8| b >= 1); // max_mm_bps ≥ 1
    let mm: u8 = kani::any_where(|&b: &u8| b <= max_mm); // mm_bps ≤ max_mm_bps
    let n: u8 = kani::any();       // scaled notional

    // Widen to u16 to hold products without overflow
    let nav16    = nav as u16;
    let rbps16   = rbps as u16;
    let max_mm16 = max_mm as u16;
    let mm16     = mm as u16;
    let n16      = n as u16;

    let nav_rbps = nav16 * rbps16;   // budget numerator
    let n_max_mm = n16 * max_mm16;   // cap numerator

    // Core assumption: notional satisfies the cap (multiplication form)
    kani::assume(n_max_mm <= nav_rbps);

    // mm ≤ max_mm ⟹ n * mm ≤ n * max_mm ≤ nav * rbps
    let n_mm = n16 * mm16;
    assert!(
        n_mm <= nav_rbps,
        "worst_loss numerator (n * mm) <= budget numerator (nav * rbps)"
    );
    // Monotone division by 10_000 then gives worst_loss ≤ budget.
    // Division reasoning is covered by Lean `single_position_bound`.
}

#[kani::proof]
fn proof_curve_ratios_are_bounded() {
    let p_lo: u64 = kani::any();
    let width: u64 = kani::any();
    let pi: u64 = kani::any();

    kani::assume(p_lo <= 1_000_000_000_000);
    kani::assume((1..=1_000_000_000_000).contains(&width));
    let p_hi = p_lo + width;

    let long = curve::long_fill_ratio(p_lo, p_hi, pi as u128).unwrap();
    let short = curve::short_fill_ratio(p_lo, p_hi, pi as u128).unwrap();

    assert!((0..=curve::FIXED_ONE).contains(&long));
    assert!((0..=curve::FIXED_ONE).contains(&short));
}

#[kani::proof]
fn proof_fee_for_notional_never_exceeds_notional_under_100_percent() {
    let notional_small: u32 = kani::any();
    let fee_micros: u32 = kani::any();
    let notional = notional_small as u64;

    kani::assume(fee_micros <= 1_000_000);

    let fee = fees::fee_for_notional(notional, fee_micros).unwrap();
    assert!(fee <= notional);
}

#[kani::proof]
fn proof_epsilon_never_exceeds_max() {
    let override_bps: u32 = kani::any();
    let dynamic_scale: u32 = kani::any();
    let allocated_lp_bps: u16 = kani::any();
    let total_assets: u64 = kani::any();
    let max_epsilon_bps: u32 = kani::any();

    kani::assume(allocated_lp_bps <= 10_000);

    let eps = epsilon::effective_epsilon_bps(
        override_bps,
        dynamic_scale,
        allocated_lp_bps,
        total_assets,
        max_epsilon_bps,
    )
    .unwrap();
    assert!(eps <= max_epsilon_bps);
}

#[kani::proof]
fn proof_lp_withdraw_payout_is_bounded_by_available_assets() {
    let total_assets_small: u8 = kani::any();
    let total_shares_small: u8 = kani::any();
    let outstanding_winner_pnl_small: u8 = kani::any();
    let shares_small: u8 = kani::any();

    let total_assets = total_assets_small as u64;
    let total_shares = total_shares_small as u64;
    let outstanding_winner_pnl = outstanding_winner_pnl_small as u64;
    let shares = shares_small as u64;

    kani::assume(total_shares > 0);
    kani::assume(outstanding_winner_pnl <= total_assets);
    kani::assume(shares <= total_shares);

    let vault = VaultModel {
        total_assets,
        total_shares,
        outstanding_winner_pnl,
        bad_debt_reserve: 0,
    };
    let out = accounting::usdc_for_shares(&vault, shares).unwrap();
    assert!(out <= vault.available_assets());
}

#[kani::proof]
fn proof_post_withdraw_health_accepts_empty_position_set() {
    let post_collateral: u64 = kani::any();
    let mm_bps: u16 = kani::any();

    assert!(margin::check_post_withdraw_health(post_collateral, &[], mm_bps).is_ok());
}

#[kani::proof]
fn proof_sour_curve_lp_loss_counts_half_unit_long_capacity() {
    let qmax_storage = (curve::FIXED_ONE as u64) / 2;
    let entry = 100_000_000;
    let p_lo = 90_000_000;
    let p_hi = 110_000_000;

    let current =
        margin::curve_max_lp_loss_per_position_micros(qmax_storage, entry, p_lo, p_hi, false)
            .unwrap();

    assert_eq!(current, 5_000_000);
}

#[kani::proof]
fn proof_sour_curve_lp_loss_counts_half_unit_short_capacity() {
    let qmax_storage = (curve::FIXED_ONE as u64) / 2;
    let entry = 100_000_000;
    let p_lo = 90_000_000;
    let p_hi = 110_000_000;

    let current =
        margin::curve_max_lp_loss_per_position_micros(qmax_storage, entry, p_lo, p_hi, true)
            .unwrap();

    assert_eq!(current, 5_000_000);
}

#[kani::proof]
fn proof_sour_curve_lp_loss_matches_exact_fractional_capacity() {
    let qmax_steps: u8 = kani::any();
    let adverse: u16 = kani::any();
    let is_short: bool = kani::any();

    kani::assume(qmax_steps > 0);
    kani::assume(adverse > 0);

    let qmax_storage = ((curve::FIXED_ONE as u64) / 256) * qmax_steps as u64;
    let entry = 100_000_000u64;
    let adverse = adverse as u64;
    let p_lo = entry - adverse;
    let p_hi = entry + adverse;

    let current =
        margin::curve_max_lp_loss_per_position_micros(qmax_storage, entry, p_lo, p_hi, is_short)
            .unwrap();
    let precise = crate::exact::curve_max_lp_loss_fractional_micros(
        qmax_storage,
        entry,
        p_lo,
        p_hi,
        is_short,
    )
    .unwrap();

    assert_eq!(current, precise);
}

#[kani::proof]
fn proof_positive_close_pnl_rejects_when_lp_assets_are_insufficient() {
    let total_assets_small: u32 = kani::any();
    let extra_pnl_small: u32 = kani::any();
    let total_assets = total_assets_small as u64;
    let final_pnl = (total_assets + extra_pnl_small as u64) as i64;

    kani::assume(total_assets > 0);
    kani::assume(extra_pnl_small > 0);

    let mut vault = VaultModel {
        total_assets,
        total_shares: total_assets,
        outstanding_winner_pnl: 0,
        bad_debt_reserve: 0,
    };
    let before = vault;

    let result = accounting::apply_close_pnl(&mut vault, 0, 0, final_pnl);

    assert!(result.is_err());
    assert_eq!(vault, before);
}

#[kani::proof]
fn proof_positive_close_pnl_debits_lp_exactly_when_fully_backed() {
    let total_assets_small: u32 = kani::any();
    let outstanding_winner_pnl_small: u32 = kani::any();
    let realized_pnl_old_small: u32 = kani::any();
    let final_pnl_small: u32 = kani::any();
    let trader_collateral_small: u32 = kani::any();

    let total_assets = total_assets_small as u64;
    let outstanding_winner_pnl = outstanding_winner_pnl_small as u64;
    let realized_pnl_old = realized_pnl_old_small as u64;
    let final_pnl = final_pnl_small as u64;
    let trader_collateral = trader_collateral_small as u64;

    kani::assume(total_assets > 0);
    kani::assume(final_pnl > 0);
    kani::assume(final_pnl <= total_assets);
    kani::assume(outstanding_winner_pnl <= total_assets);
    kani::assume(realized_pnl_old <= outstanding_winner_pnl);

    let mut vault = VaultModel {
        total_assets,
        total_shares: total_assets,
        outstanding_winner_pnl,
        bad_debt_reserve: 0,
    };
    let before = vault;

    let trader_after = accounting::apply_close_pnl(
        &mut vault,
        trader_collateral,
        realized_pnl_old as i64,
        final_pnl as i64,
    )
    .unwrap();

    assert_eq!(trader_after - trader_collateral, final_pnl);
    assert_eq!(before.total_assets - vault.total_assets, final_pnl);
    assert_eq!(
        vault.outstanding_winner_pnl,
        before.outstanding_winner_pnl - realized_pnl_old
    );
}

// ---------------------------------------------------------------------------
// v0.5.1-P1c — aggregate-budget enforcement proofs
//
// Three invariants over the new `Protocol.aggregate_max_lp_loss` hot-path
// counter, the per-market `Market::worst_case_lp_loss` exposure formula, and
// the upsert-time enforcement check at `upsert_position.rs:648`:
//
//   I. Cap holds across a single upsert: if the precondition
//      `aggregate_pre <= cap` and the on-chain check
//      `proposed_aggregate <= cap` both hold, then `aggregate_post <= cap`
//      after `update_aggregate(delta)`.
//
//   II. `recompute_aggregate` is byte-exact a sum of per-market
//       `worst_case_lp_loss` terms (idempotence + no overflow under bounded
//       inputs). N=4 markets — sized to fit CBMC's bitvector budget.
//
//   III. `update_aggregate` rejects rather than wraps when a negative
//        delta would push the counter below 0 (the on-chain
//        `AggregateBudgetUnderflow` Custom:43 path).
//
// Decidability strategy mirrors the v0.5.0 LP-scaled cap proof: u8/u16
// inputs so symbolic products fit u32/u64 and CBMC terminates within the
// 45s harness budget. Bound choices documented per harness.
// ---------------------------------------------------------------------------

use crate::aggregate::{
    aggregate_cap, recompute_aggregate, AggregateError, MarketModel, ProtocolModel,
};

/// Invariant I — cap holds under a single `update_aggregate(delta)`.
///
/// Models the upsert sequence:
///   1. Snapshot `aggregate_pre = protocol.aggregate_max_lp_loss`.
///   2. Compute `aggregate_cap` from current vault + budget knobs.
///   3. Assume system entry invariant: `aggregate_pre <= cap`.
///   4. Assume the on-chain `require!` at upsert_position.rs:648 passed
///      (i.e., `proposed_aggregate >= 0 && proposed <= cap`).
///   5. Apply `update_aggregate(delta)` (signed-add).
///   6. Assert post-condition: `aggregate_post <= cap`.
///
/// CBMC budget: `total_assets` bounded to u8 (so `total_assets × 10_000`
/// fits in u32), `aggregate_budget_bps` and `max_mm_bps` bounded to u8
/// (1..=255). Same trick as `proof_per_user_cap_bounds_single_position_loss`.
#[kani::proof]
#[kani::unwind(2)]
fn proof_aggregate_cap_invariant_holds_under_update() {
    // Bounded knobs — keeps cap arithmetic decidable.
    let total_assets_small: u8 = kani::any();
    let agg_bps_small: u8 = kani::any_where(|&b: &u8| b >= 1);
    let max_mm_small: u8 = kani::any_where(|&b: &u8| b >= 1);

    let total_assets = total_assets_small as u64;
    let aggregate_budget_bps = agg_bps_small as u16;
    let max_mm_bps = max_mm_small as u16;

    // Cap = total_assets × budget / max_mm. Under u8 inputs:
    //   max cap = 255 × 255 / 1 = 65_025  (fits u32).
    let cap = aggregate_cap(total_assets, aggregate_budget_bps, max_mm_bps);

    // System-entry invariant: aggregate_pre satisfies the cap.
    let aggregate_pre_small: u32 = kani::any();
    let aggregate_pre = aggregate_pre_small as u128;
    kani::assume(aggregate_pre <= cap);

    let mut protocol = ProtocolModel {
        aggregate_max_lp_loss: aggregate_pre,
        aggregate_budget_bps,
        max_mm_bps,
    };

    // Symbolic delta — bounded i32 so the i128 add is decidable. Covers
    // both opens (positive delta) and closes/cascades (negative delta).
    let delta_small: i32 = kani::any();
    let delta = delta_small as i128;

    // Mirror the on-chain require!: assume the upsert check passed.
    let proposed_signed = (aggregate_pre as i128).wrapping_add(delta);
    kani::assume(proposed_signed >= 0);
    let proposed = proposed_signed as u128;
    kani::assume(proposed <= cap);

    // Apply the update. With the precondition above, both Overflow and
    // Underflow paths are ruled out, so this must succeed.
    let r = protocol.update_aggregate(delta);
    assert!(r.is_ok());

    // Post-condition: counter still respects the cap (cap depends only on
    // total_assets / budget / max_mm, none of which the helper mutates,
    // so cap_pre == cap_post within a single upsert).
    assert!(protocol.aggregate_max_lp_loss <= cap);
}

/// Invariant II — `recompute_aggregate` IS the sum of per-market
/// `worst_case_lp_loss` terms (idempotence + no overflow under bounded inputs).
///
/// Models the body of `recompute_aggregate.rs::handler` (modulo the runtime
/// PDA + program-owner check, which is account-authentication, not math).
/// N=4 markets — large enough to exercise the fold, small enough that 4×
/// (u8 × u8 × u8) products stay decidable.
///
/// CBMC budget: per-market scalar inputs bounded to u8. Worst-case per-term
/// product is 255 × 255 × 255 = 16_581_375 (fits u32, well under u128 wrap).
/// Sum of four such terms ≤ 66_325_500 (still u32). No `saturating_*` arm
/// of the underlying formula can be reached, so the assertion of equality
/// against the explicit fold is sound.
#[kani::proof]
#[kani::unwind(5)]
fn proof_recompute_matches_per_market_sum_n4() {
    let max_mm_small: u8 = kani::any_where(|&b: &u8| b >= 1);
    let max_mm_bps = max_mm_small as u16;

    // Construct 4 bounded-symbolic markets. We use u8 for each numeric
    // field so 4× fold stays inside CBMC's bitvector budget.
    let mk = |long_oi_s: u8,
              short_oi_s: u8,
              curve_long_s: u8,
              curve_short_s: u8,
              price_s: u8|
     -> MarketModel {
        MarketModel {
            long_oi: long_oi_s as u64,
            short_oi: short_oi_s as u64,
            curve_max_lp_loss_long: curve_long_s as u128,
            curve_max_lp_loss_short: curve_short_s as u128,
            price_smoothed: price_s as u128,
        }
    };

    let m0 = mk(
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
    );
    let m1 = mk(
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
    );
    let m2 = mk(
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
    );
    let m3 = mk(
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
        kani::any(),
    );

    let markets = [m0, m1, m2, m3];

    let recomputed = recompute_aggregate(&markets, max_mm_bps).expect("no overflow at u8 bounds");

    // Independent fold of per-market terms — must equal the helper's own
    // accumulator. This is the idempotence + decomposition property: the
    // permissionless `recompute_aggregate` ix produces the same value as
    // summing `worst_case_lp_loss` market-by-market with the same inputs.
    let t0 = m0.worst_case_lp_loss(max_mm_bps, m0.price_smoothed);
    let t1 = m1.worst_case_lp_loss(max_mm_bps, m1.price_smoothed);
    let t2 = m2.worst_case_lp_loss(max_mm_bps, m2.price_smoothed);
    let t3 = m3.worst_case_lp_loss(max_mm_bps, m3.price_smoothed);
    let expected: u128 = t0 + t1 + t2 + t3;

    assert_eq!(recomputed, expected);
}

/// Invariant III — `update_aggregate` rejects rather than wraps when a
/// negative delta would push the counter below 0.
///
/// Mirrors the on-chain `AggregateBudgetUnderflow` (Custom:43) path in
/// `state.rs::Protocol::update_aggregate`. Defensive guard: cascade
/// decrements should always be ≤ the matching open's increment by
/// construction, but a drift bug must surface as Err rather than wrap.
///
/// CBMC budget: `aggregate_pre` and `delta` bounded to u32/i32 — same
/// trick as the cap-invariant proof. The state-mutation no-op-on-Err
/// claim is also verified.
#[kani::proof]
#[kani::unwind(2)]
fn proof_update_aggregate_no_underflow() {
    let aggregate_pre_small: u32 = kani::any();
    let delta_small: i32 = kani::any();
    let aggregate_pre = aggregate_pre_small as u128;
    let delta = delta_small as i128;

    let mut protocol = ProtocolModel {
        aggregate_max_lp_loss: aggregate_pre,
        aggregate_budget_bps: 10_000,
        max_mm_bps: 100,
    };
    let before = protocol;

    // Restrict to the underflow regime: prior + delta is representable in
    // i128 (always true at our bounded sizes) but is negative.
    let signed_sum = (aggregate_pre as i128) + delta;
    kani::assume(signed_sum < 0);

    let result = protocol.update_aggregate(delta);
    assert_eq!(result, Err(AggregateError::Underflow));
    // Counter must NOT have been mutated on the Err path.
    assert_eq!(protocol, before);
}

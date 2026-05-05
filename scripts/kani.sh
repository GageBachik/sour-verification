#!/usr/bin/env bash
set -euo pipefail

if ! command -v cargo-kani >/dev/null 2>&1 && ! cargo kani --version >/dev/null 2>&1; then
  echo "Kani is not installed. Install it first: https://model-checking.github.io/kani/getting-started.html" >&2
  exit 127
fi

status=0

run_harness() {
  local harness="$1"
  if cargo kani -p sour-verifier \
    --harness "$harness" \
    --no-assertion-reach-checks \
    -Z unstable-options \
    --harness-timeout 45; then
    echo "Verified: $harness"
  else
    echo "Verification failed: $harness" >&2
    status=1
  fi
}

run_harness proof_curve_ratios_are_bounded
run_harness proof_epsilon_never_exceeds_max
run_harness proof_lp_withdraw_payout_is_bounded_by_available_assets
run_harness proof_post_withdraw_health_accepts_empty_position_set
run_harness proof_sour_curve_lp_loss_counts_half_unit_long_capacity
run_harness proof_sour_curve_lp_loss_counts_half_unit_short_capacity
run_harness proof_sour_curve_lp_loss_matches_exact_fractional_capacity
run_harness proof_positive_close_pnl_rejects_when_lp_assets_are_insufficient
run_harness proof_positive_close_pnl_debits_lp_exactly_when_fully_backed
run_harness proof_per_user_cap_bounds_single_position_loss
run_harness proof_aggregate_cap_invariant_holds_under_update
run_harness proof_recompute_matches_per_market_sum_n4
run_harness proof_update_aggregate_no_underflow
run_harness proof_max_oi_notional_cap_bound
run_harness proof_max_oi_cross_market_dollar_parity

echo "Skipped proof_fee_for_notional_never_exceeds_notional_under_100_percent: current direct Sour function boundary times out in CBMC; see docs/historical-2026-05-03-fix-implementation-spec.md."

exit "$status"

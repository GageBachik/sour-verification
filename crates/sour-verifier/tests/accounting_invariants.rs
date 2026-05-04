use proptest::prelude::*;
use sour_math::{
    auth, clear,
    curve::{self, FIXED_ONE},
    epsilon, fees, gap_velocity, margin,
    solver::{self, CurveSlim},
};
use sour_verifier::{
    accounting::{apply_close_pnl, deposit_lp, withdraw_lp, VaultModel},
    exact,
};

const ONE_USDC: u128 = 1_000_000;

fn qstore(base_units: u64) -> u64 {
    ((base_units as u128) * (FIXED_ONE as u128)) as u64
}

proptest! {
    #[test]
    fn curve_fill_ratios_are_bounded_and_monotone(
        p_lo in 1u64..1_000_000,
        width in 1u64..1_000_000,
        pi_a in 0u64..2_500_000,
        pi_b in 0u64..2_500_000,
    ) {
        let p_hi = p_lo + width;
        let lo = pi_a.min(pi_b) as u128;
        let hi = pi_a.max(pi_b) as u128;

        let long_lo = curve::long_fill_ratio(p_lo, p_hi, lo).unwrap();
        let long_hi = curve::long_fill_ratio(p_lo, p_hi, hi).unwrap();
        let short_lo = curve::short_fill_ratio(p_lo, p_hi, lo).unwrap();
        let short_hi = curve::short_fill_ratio(p_lo, p_hi, hi).unwrap();

        prop_assert!((0..=FIXED_ONE).contains(&long_lo));
        prop_assert!((0..=FIXED_ONE).contains(&long_hi));
        prop_assert!((0..=FIXED_ONE).contains(&short_lo));
        prop_assert!((0..=FIXED_ONE).contains(&short_hi));
        prop_assert!(long_lo >= long_hi);
        prop_assert!(short_lo <= short_hi);
    }

    #[test]
    fn notional_from_fill_ignores_side_sign(
        size in 0i128..(1i128 << 80),
        pi_star in 0u128..1_000_000_000_000u128,
    ) {
        let pos = fees::notional_from_fill(size, pi_star);
        let neg = fees::notional_from_fill(-size, pi_star);
        prop_assert_eq!(pos, neg);
    }

    #[test]
    fn fee_for_notional_never_exceeds_notional_when_rate_is_at_most_100_percent(
        notional in 0u64..u64::MAX,
        fee_micros in 0u32..=1_000_000u32,
    ) {
        let fee = fees::fee_for_notional(notional, fee_micros).unwrap();
        prop_assert!(fee <= notional);
    }

    #[test]
    fn skew_aware_fee_is_bounded_for_valid_trade_dirs(
        notional in 0u64..1_000_000_000_000u64,
        fee_micros in 0u32..=1_000_000u32,
        long_oi in 0u64..1_000_000_000_000u64,
        short_oi in 0u64..1_000_000_000_000u64,
        dir_is_long in any::<bool>(),
    ) {
        let trade_dir = if dir_is_long { 1 } else { -1 };
        let raw = fees::fee_for_notional(notional, fee_micros).unwrap();
        let skewed = fees::skew_aware_fee(notional, fee_micros, long_oi, short_oi, trade_dir).unwrap();
        let upper = ((raw as u128) * 1_500_000u128 / 1_000_000u128) as u64;
        prop_assert!(skewed <= upper);
    }

    #[test]
    fn epsilon_bps_is_always_capped(
        override_bps in 0u32..2_000_000,
        dynamic_scale in 0u32..2_000_000,
        allocated_lp_bps in 0u16..=10_000,
        total_assets in 0u64..u64::MAX,
        max_epsilon_bps in 0u32..500_000,
    ) {
        let bps = epsilon::effective_epsilon_bps(
            override_bps,
            dynamic_scale,
            allocated_lp_bps,
            total_assets,
            max_epsilon_bps,
        ).unwrap();
        prop_assert!(bps <= max_epsilon_bps);
    }

    #[test]
    fn gap_velocity_clamps_to_configured_band(
        last in 1u128..1_000_000_000_000u128,
        raw in 0u128..2_000_000_000_000u128,
        cap_bps in 1u16..10_000u16,
    ) {
        let out = gap_velocity::clamp_gap_velocity(last, raw, cap_bps);
        let max_step = last.saturating_mul(cap_bps as u128) / 10_000u128;
        prop_assert!(out >= last.saturating_sub(max_step));
        prop_assert!(out <= last.saturating_add(max_step));
    }

    #[test]
    fn outstanding_winner_delta_matches_positive_pnl_clamp(
        old_pnl in i64::MIN..i64::MAX,
        new_pnl in i64::MIN..i64::MAX,
    ) {
        let delta = clear::outstanding_winner_pnl_delta(old_pnl, new_pnl);
        let expected = new_pnl.max(0) - old_pnl.max(0);
        prop_assert_eq!(delta, expected);
    }

    #[test]
    fn vault_lp_deposit_and_withdraw_preserve_solvency(
        initial_assets in 1u64..1_000_000_000_000,
        outstanding in 0u64..500_000_000_000,
        deposit in 1u64..1_000_000_000,
        withdraw_shares in 1u64..1_000_000_000,
    ) {
        let mut vault = VaultModel {
            total_assets: initial_assets,
            total_shares: initial_assets,
            outstanding_winner_pnl: outstanding.min(initial_assets),
            bad_debt_reserve: 0,
        };
        let minted = deposit_lp(&mut vault, deposit).unwrap();
        prop_assert!(minted > 0);
        let burn = withdraw_shares.min(vault.total_shares);
        let out = withdraw_lp(&mut vault, burn).unwrap();
        prop_assert!(out <= initial_assets.saturating_add(deposit));
        prop_assert!(vault.available_assets() <= vault.total_assets);
    }
}

#[test]
fn solver_rejects_unsorted_breakpoints() {
    let curves = [CurveSlim {
        is_short: false,
        qmax: qstore(1),
        p_lo: 90,
        p_hi: 110,
    }];
    assert!(solver::solve_pi_star(&curves, &[110, 90], 100, 1).is_err());
}

#[test]
fn solver_lhs_is_non_increasing_on_valid_curve_books() {
    let curves = [
        CurveSlim {
            is_short: false,
            qmax: qstore(3),
            p_lo: 90,
            p_hi: 110,
        },
        CurveSlim {
            is_short: true,
            qmax: qstore(2),
            p_lo: 95,
            p_hi: 120,
        },
    ];
    let a = solver::lhs(&curves, 100, 105, 10).unwrap();
    let b = solver::lhs(&curves, 106, 105, 10).unwrap();
    assert!(a >= b, "LHS must be monotone non-increasing: {a} < {b}");
}

#[test]
fn cross_margin_withdrawal_gate_rejects_post_withdraw_under_maintenance() {
    let size = (FIXED_ONE * FIXED_ONE) * 10;
    let snaps = [margin::PositionSnap {
        realized_pnl: 0,
        effective_size_signed: size,
        pi_star: 100 * ONE_USDC,
        index: 0,
    }];
    assert!(margin::check_post_withdraw_health(1, &snaps, 100).is_err());
}

#[test]
fn cascade_zeroes_worst_losers_until_remaining_account_is_healthy() {
    let size = (FIXED_ONE * FIXED_ONE) * 10;
    let positions = [
        clear::CascadePos {
            index: 10,
            realized_pnl: -1_000_000,
            effective_size_signed: size,
            pi_star: 100 * ONE_USDC,
            maintenance_margin_bps: 100,
        },
        clear::CascadePos {
            index: 11,
            realized_pnl: 0,
            effective_size_signed: size,
            pi_star: 100 * ONE_USDC,
            maintenance_margin_bps: 100,
        },
    ];
    let zeroed = clear::select_zeroed_indices(&positions, 20_000_000, 100).unwrap();
    assert_eq!(zeroed, vec![10]);
}

#[test]
fn close_positive_pnl_debits_lp_and_credits_trader() {
    let mut vault = VaultModel {
        total_assets: 100_000_000,
        total_shares: 100_000_000,
        outstanding_winner_pnl: 10_000_000,
        bad_debt_reserve: 0,
    };
    let trader_after = apply_close_pnl(&mut vault, 5_000_000, 10_000_000, 7_000_000).unwrap();
    assert_eq!(trader_after, 12_000_000);
    assert_eq!(vault.total_assets, 93_000_000);
    assert_eq!(vault.outstanding_winner_pnl, 0);
}

#[test]
fn auth_rejects_zero_delegate_for_zero_signer() {
    let owner = [1u8; 32];
    let zero = [0u8; 32];
    assert!(auth::check_authorized(&owner, &zero, &zero).is_err());
}

#[test]
fn exact_curve_lp_loss_counts_fractional_qmax() {
    let qmax_storage = (FIXED_ONE as u64) / 2;
    let entry = 100_000_000u64;
    let p_hi = 110_000_000u64;
    let precise =
        exact::curve_max_lp_loss_fractional_micros(qmax_storage, entry, 90_000_000, p_hi, false)
            .unwrap();
    assert_eq!(precise, 5_000_000);
}

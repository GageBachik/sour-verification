use sour_math::{curve::FIXED_ONE, margin::curve_max_lp_loss_per_position_micros};
use sour_verifier::{
    accounting::{apply_close_pnl, VaultModel},
    exact,
};

#[test]
fn sour_curve_lp_loss_cap_counts_fractional_qmax_long_capacity() {
    let qmax_storage = (FIXED_ONE as u64) / 2;
    let entry = 100_000_000u64;
    let p_lo = 90_000_000u64;
    let p_hi = 110_000_000u64;

    let current =
        curve_max_lp_loss_per_position_micros(qmax_storage, entry, p_lo, p_hi, false).unwrap();
    let exact =
        exact::curve_max_lp_loss_fractional_micros(qmax_storage, entry, p_lo, p_hi, false).unwrap();

    assert_eq!(exact, 5_000_000);
    assert_eq!(
        current, exact,
        "CURVE LP capacity must keep fractional qmax_storage until after multiplying by long adverse price distance"
    );
}

#[test]
fn sour_curve_lp_loss_cap_counts_fractional_qmax_short_capacity() {
    let qmax_storage = (FIXED_ONE as u64) / 2;
    let entry = 100_000_000u64;
    let p_lo = 90_000_000u64;
    let p_hi = 110_000_000u64;

    let current =
        curve_max_lp_loss_per_position_micros(qmax_storage, entry, p_lo, p_hi, true).unwrap();
    let exact =
        exact::curve_max_lp_loss_fractional_micros(qmax_storage, entry, p_lo, p_hi, true).unwrap();

    assert_eq!(exact, 5_000_000);
    assert_eq!(
        current, exact,
        "CURVE LP capacity must keep fractional qmax_storage until after multiplying by short adverse price distance"
    );
}

#[test]
fn close_positive_pnl_rejects_when_lp_assets_are_insufficient() {
    let mut vault = VaultModel {
        total_assets: 1_000_000,
        total_shares: 1_000_000,
        outstanding_winner_pnl: 0,
        bad_debt_reserve: 0,
    };
    let before = vault;

    let result = apply_close_pnl(&mut vault, 0, 0, 5_000_000);

    assert!(
        result.is_err(),
        "positive close PnL must reject when LP assets cannot exactly fund the payout"
    );
    assert_eq!(
        vault, before,
        "failed close must not mutate vault accounting"
    );
}

#[test]
fn close_positive_pnl_debits_lp_exactly_when_fully_backed() {
    let mut vault = VaultModel {
        total_assets: 10_000_000,
        total_shares: 10_000_000,
        outstanding_winner_pnl: 2_000_000,
        bad_debt_reserve: 0,
    };
    let trader_before = 7_000;

    let trader_after = apply_close_pnl(&mut vault, trader_before, 750_000, 1_250_000).unwrap();

    assert_eq!(trader_after - trader_before, 1_250_000);
    assert_eq!(vault.total_assets, 8_750_000);
    assert_eq!(vault.outstanding_winner_pnl, 1_250_000);
}

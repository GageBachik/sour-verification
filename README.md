# Sour Verification

Standalone verification workspace for the Sour flow-perps program.

This repo intentionally separates three concerns:

- `crates/sour-verifier`: executable Rust/property tests and Kani proof harnesses over Sour's pure math/accounting surface.
- `lean/`: theorem-level economic model for no-bad-debt and settlement-ratio reasoning.
- `certora/`: CVLR rule templates for Certora Solana verification.

The Rust crate depends on the local Sour math crate at `../sour/programs/sour-math`.

## Commands

```bash
# Green executable invariant lane.
./scripts/test-green.sh

# Known breakage lane. This currently fails and demonstrates a real mismatch.
./scripts/test-breakdowns.sh

# Kani lane, requires Kani installed.
./scripts/kani.sh

# Expected-failure Kani counterexample, requires Kani installed.
./scripts/kani-expected-failures.sh
```

## Current Breakdowns

`sour_math::margin::curve_max_lp_loss_per_position_micros` floors
`qmax_storage / 2^32` before multiplying by adverse price distance.

That makes sub-base-unit CURVE positions reserve zero LP capacity even when
their fractional max loss is nonzero. The counterexample in
`tests/sour_breakdown_cases.rs` shows:

```text
qmax_storage = 0.5 * 2^32
entry        = 100 USDC
p_hi         = 110 USDC
current cap  = 0
exact cap    = 5_000_000 micro-USDC
```

The proof-driven fix should move Sour's helper to:

```text
qmax_storage * adverse_price_distance / 2^32
```

instead of:

```text
(qmax_storage / 2^32) * adverse_price_distance
```

The second executable counterexample mirrors `close_position`'s positive-PnL
path: trader collateral is credited by full PnL while LP assets use
`saturating_sub`. With `total_assets = 1_000_000` and `final_pnl = 5_000_000`,
trader collateral increases by `5_000_000` while LP assets decrease by only
`1_000_000`.

## Instruction-Level Targets

The next Certora/e2e layer should cover:

- Remaining-account authenticity for `clear_batch` and `withdraw_collateral`.
- Oracle account binding against `market.price_account_0..N`, not owner-only checks.
- Active re-upsert OI/cap/collateral-lock deltas.
- `cross_im_used <= trader.usdc_collateral` after fees.
- Full pooled-vault conservation across LP assets, bad-debt reserve, trader collateral, and isolated locks.

# Sour Verification

Formal proofs and executable invariants for [Sour Protocol](https://sour.finance) — a flow-perpetuals DEX on Solana.

We do security-by-obscurity for the implementation (the program lives in a private monorepo) and radical transparency on the math (this repo is public). Every load-bearing accounting invariant — pooled-vault conservation, LP-scaled per-user cap, aggregate-budget cap, fractional CURVE LP capacity, positive-PnL conservation on close, price-agnostic per-market OI cap — is mirrored byte-for-byte from the on-chain handlers and proven in [Kani](https://model-checking.github.io/kani/) (bounded model checking) and / or [Lean 4](https://leanprover.github.io/) (interactive theorem proving). Both tools verify the proofs match the as-built code, not the as-written spec.

The proofs are not "this contract cannot ever be hacked." They are "for these named invariants, no input — symbolic over a bounded domain or unbounded over Nat — can break them." Read the [What's NOT Proven](#whats-not-proven) section to see exactly where the boundary sits.

## What's proven

| Invariant | Source | Tool |
|-----------|--------|------|
| Curve fill ratios stay in `[0, 1]` | `crates/sour-verifier/src/kani_harnesses.rs::proof_curve_ratios_are_bounded` | Kani |
| Epsilon never exceeds `max_epsilon_bps` | `crates/sour-verifier/src/kani_harnesses.rs::proof_epsilon_never_exceeds_max` | Kani |
| LP withdraw payout bounded by available assets | `crates/sour-verifier/src/kani_harnesses.rs::proof_lp_withdraw_payout_is_bounded_by_available_assets` | Kani |
| Empty position set passes withdraw health check | `crates/sour-verifier/src/kani_harnesses.rs::proof_post_withdraw_health_accepts_empty_position_set` | Kani |
| CURVE LP loss counts half-unit fractional `qmax` (long) | `crates/sour-verifier/src/kani_harnesses.rs::proof_sour_curve_lp_loss_counts_half_unit_long_capacity` | Kani |
| CURVE LP loss counts half-unit fractional `qmax` (short) | `crates/sour-verifier/src/kani_harnesses.rs::proof_sour_curve_lp_loss_counts_half_unit_short_capacity` | Kani |
| CURVE LP loss matches exact fractional capacity at all `qmax` steps | `crates/sour-verifier/src/kani_harnesses.rs::proof_sour_curve_lp_loss_matches_exact_fractional_capacity` | Kani |
| Positive close PnL rejects when LP assets insufficient | `crates/sour-verifier/src/kani_harnesses.rs::proof_positive_close_pnl_rejects_when_lp_assets_are_insufficient` | Kani |
| Positive close PnL debits LP exactly when fully backed | `crates/sour-verifier/src/kani_harnesses.rs::proof_positive_close_pnl_debits_lp_exactly_when_fully_backed` | Kani |
| Per-user cap bounds single-position worst LP loss (numerator) | `crates/sour-verifier/src/kani_harnesses.rs::proof_per_user_cap_bounds_single_position_loss` | Kani |
| Per-user cap bounds single-position worst LP loss (full division form, general Nat) | `lean/SourVerification/Sour/NoBadDebt.lean::single_position_bound` | Lean |
| Aggregate-cap invariant holds under `Protocol::update_aggregate(delta)` | `crates/sour-verifier/src/kani_harnesses.rs::proof_aggregate_cap_invariant_holds_under_update` | Kani |
| `recompute_aggregate` byte-equals `Σ Market::worst_case_lp_loss` (N=4) | `crates/sour-verifier/src/kani_harnesses.rs::proof_recompute_matches_per_market_sum_n4` | Kani |
| `update_aggregate` returns `Err(Underflow)` rather than wrapping (counter unchanged on Err) | `crates/sour-verifier/src/kani_harnesses.rs::proof_update_aggregate_no_underflow` | Kani |
| Aggregate worst loss = sum of per-market worst-cases (general N over Nat, no Mathlib) | `lean/SourVerification/Sour/NoBadDebt.lean::aggregate_bound_general_n` | Lean |
| Aggregate cap preserved under `update_aggregate` when upsert check passed | `lean/SourVerification/Sour/NoBadDebt.lean::update_aggregate_preserves_bound` | Lean |
| Per-market `max_oi` cap bounds gross-side notional (admit ⟹ side_notional ≤ cap; reject ⟹ side_notional > cap) | `crates/sour-verifier/src/kani_harnesses.rs::proof_max_oi_notional_cap_bound` | Kani |
| `max_oi` cap is price-agnostic — same dollar threshold across markets at any mark price (concrete BTC vs XRP) | `crates/sour-verifier/src/kani_harnesses.rs::proof_max_oi_cross_market_dollar_parity` | Kani |
| `max_oi` notional cap bound + cap monotonicity + concrete BTC/XRP dollar-parity (general Nat, no Mathlib) | `lean/SourVerification/Sour/NoBadDebt.lean::max_oi_bound` | Lean |

Plus 8 executable property tests in `crates/sour-verifier/tests/accounting_invariants.rs` (curve monotonicity, fee bound, skew-aware fee bound, epsilon cap, gap-velocity clamp, winner-PnL delta accounting, LP deposit / withdraw solvency), 4 deterministic counterexample regressions in `crates/sour-verifier/tests/sour_breakdown_cases.rs` that lock in the historical CURVE-cap and positive-PnL bug fixes (formerly red-light proofs, now green regression gates after Sour shipped the fixes), and 4 unit tests for the `max_oi` notional-micros model in `crates/sour-verifier/src/max_oi.rs`.

## What's NOT proven

- **General-N aggregate decomposition over `Finset`**. The Lean `aggregate_bound_general_n` proof is general over `Nat` and a flat `List`, which is enough to certify `recompute_aggregate` against the on-chain fold. A purely-symbolic `Finset.sum_le_card_nsmul`-style theorem would need [Mathlib](https://leanprover-community.github.io/mathlib4_docs/) and is deferred. Concrete N=4 is verified by both `aggregate_bound_n4_decomposes` (Lean, `decide`) and `proof_recompute_matches_per_market_sum_n4` (Kani).
- **`fee_for_notional` direct CBMC proof**. The arithmetic involves `u128 / 1_000_000` which CBMC cannot decide inside the harness time budget. The property is covered by an executable proptest in `accounting_invariants.rs` (10K random cases per build) and by a CVLR rule template in `certora/sour-cvlr/src/lib.rs`. The `kani.sh` script intentionally skips the symbolic harness and explains why.
- **Some Kani harnesses bound inputs to u8 or u32**. Where CBMC's bitvector search blows past the 45 s harness budget at full u64 / u128 width, we narrow the symbolic domain to the smallest bit-width that still exercises the ratio structure. The `max_oi` notional-cap harness narrows to u8 inputs, the per-user-cap numerator harness narrows similarly, and so on. The pure-arithmetic ratio identities verified at narrower widths are sound at full width by monotonicity, but we have not mechanized the lift; treat narrow-width Kani proofs as bounded-domain. The full-width helpers are exercised by the accompanying unit tests in the same module.
- **Account authentication / runtime concerns**. Pyth account binding, raw-byte remaining-account discriminator + program-owner + PDA-re-derivation checks, and CPI authority signing live at the Solana runtime layer and are out of scope for the pure-math verifier crate. They are documented as proof obligations under "Account Authenticity" in `docs/instruction-verification-targets.md` and currently covered only by Sour's surfpool integration suite (private repo).
- **No external audit (yet)**. This repo lets you reproduce the proofs the protocol team has shipped. It is not a substitute for an independent third-party audit.

## Reproduce

Tooling versions tested locally:

- Rust + Cargo (any stable 1.7x toolchain).
- [Kani 0.67.0](https://model-checking.github.io/kani/install-guide.html) (`cargo kani`).
- [Lean 4.29.1 + Lake 5.0.0](https://leanprover.github.io/lean4/doc/quickstart.html).

```bash
git clone git@github.com:GageBachik/sour-verification.git
cd sour-verification

# Sibling repo `sour` is required for the math-crate path dependency.
# The verifier depends on `../sour/programs/sour-math`. Without it,
# `cargo build` fails to resolve the `sour-math` crate. Without
# read access to the (private) `sour` repo, only the Lean lane runs.
git clone git@github.com:GageBachik/sour.git ../sour    # if you have access
cd sour-verification

# Executable invariant lane (proptest + deterministic counterexamples).
./scripts/test-green.sh

# Counterexample-regression lane (formerly red-light, now green after fixes).
./scripts/test-breakdowns.sh

# Bounded model checking. Requires `cargo kani`. ~30 s wall (most harnesses
# decide in well under 1 s; the wider-input ones use 45 s harness timeouts).
./scripts/kani.sh

# Lean theorems. Requires `lake`. ~5 s wall on cold cache.
cd lean && lake build && cd ..

# CVLR rule template compile-check. Requires Certora install for full runs.
cargo check --manifest-path certora/sour-cvlr/Cargo.toml
```

The `scripts/kani-expected-failures.sh` script is retained as a thin wrapper around `kani.sh` for backward compatibility — the original failure lane was retired once the P0 bugs were fixed in upstream Sour and the formerly-red-light proofs went green.

## Repo layout

```text
crates/sour-verifier/
  src/accounting.rs       # Vault model + close-PnL conservation logic.
  src/aggregate.rs        # v0.5.1-P1c aggregate-budget enforcement model.
  src/exact.rs            # Exact fractional CURVE LP-cap formula (oracle).
  src/max_oi.rs           # v0.6.0 per-market max_oi notional-micros model.
  src/kani_harnesses.rs   # 16 #[kani::proof] harnesses (cfg(kani) only).
  tests/                  # proptest + deterministic regressions.

lean/
  SourVerification/Sour/NoBadDebt.lean   # Theorems over Nat, no Mathlib.
  lakefile.lean

certora/sour-cvlr/
  src/lib.rs              # CVLR rule templates mirroring Kani harnesses.
  README.md

scripts/
  test-green.sh           # cargo test -p sour-verifier --test accounting_invariants
  test-breakdowns.sh      # cargo test -p sour-verifier --test sour_breakdown_cases
  kani.sh                 # Iterates every #[kani::proof] harness.
  kani-expected-failures.sh   # Wrapper, see note above.

docs/
  instruction-verification-targets.md   # Per-instruction proof obligations.
  historical-2026-05-03-fix-implementation-spec.md   # Worked example: red-light-first hardening.
```

## License

MIT. See `LICENSE`.

## Acknowledgements

Built alongside [Sour Protocol](https://sour.finance). The verification methodology owes a debt to the [Aleo Solana CVLR rule examples](https://docs.certora.com/en/latest/), the [Kani Solana stdlib examples](https://model-checking.github.io/kani/tutorial.html), and Mathlib's purely-`Nat` proof style for the no-Mathlib Lean theorems.

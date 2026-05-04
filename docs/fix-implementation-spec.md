# Sour Verification Fix Implementation Spec

Date: 2026-05-03

Repos:

- Verification harness: `/Users/loser/projects/sour-verification`
- Target protocol: `/Users/loser/projects/sour`
- Scope note: "flow perps" means the Sour repo above only. Do not use or reference Tidepool for this work.

## Goal

Move Sour toward proof-first, red-light development for security-critical Solana smart contract behavior. The next implementation session should start from failing regression cases and make the Sour program satisfy them.

The verification suite does not prove "cannot ever be hacked." It currently does something narrower and useful:

- Captures concrete drain/accounting paths as failing tests and symbolic expected-failure proofs.
- Provides green invariants for math/accounting helpers that are already behaving.
- Defines the next security hardening work as small, subagent-owned implementation slices.

## Current Verification Evidence

Tool versions observed locally:

- Kani: `kani 0.67.0`, `cargo-kani 0.67.0`
- Lean/Lake: `Lean 4.29.1`, `Lake 5.0.0-src+f72c35b`
- Certora: `certora-cli 8.11.3`

Commands run from `/Users/loser/projects/sour-verification`:

- `./scripts/test-green.sh`
  - Result: pass, 15 tests.
  - Covers bounded curve ratios, fee bounds via Rust tests/proptests, epsilon cap, gap velocity clamp, LP share accounting, solver rejects unsorted curves, withdraw health rejects unsafe exposure, cascade zeroes worst loser, close-PnL mirror model, auth checks, and exact fractional curve cap helper.

- `./scripts/test-breakdowns.sh`
  - Result: intentionally fails 2 tests.
  - `sour_curve_lp_loss_cap_should_count_fractional_qmax_capacity`: current Sour helper returns `0`; exact helper returns `5_000_000`.
  - `close_positive_pnl_should_not_credit_more_than_lp_debits`: trader credit can be `5_000_000` while LP debit is only `1_000_000`.

- `cd lean && lake build`
  - Result: pass.
  - Lean module builds and includes concrete theorems for the curve-cap counterexample plus a pooled-vault payout bound theorem.

- `./scripts/kani.sh`
  - Result: pass.
  - Green Kani harnesses:
    - `proof_curve_ratios_are_bounded`
    - `proof_epsilon_never_exceeds_max`
    - `proof_lp_withdraw_payout_is_bounded_by_available_assets`
    - `proof_post_withdraw_health_accepts_empty_position_set`
  - Direct Kani proof for `fee_for_notional` is currently skipped by this script because proving that exact Sour function boundary times out in CBMC. This is proof-engineering debt, not an observed counterexample. Rust tests/proptests still cover the property.

- `./scripts/kani-expected-failures.sh`
  - Result: pass, because both expected failures were observed.
  - Symbolically confirms:
    - fractional `qmax_storage < FIXED_ONE` is lost by the current curve max LP loss cap.
    - positive close PnL can credit the trader more than the LP vault is debited.

- `cargo check --manifest-path certora/sour-cvlr/Cargo.toml`
  - Result: pass.
  - Confirms the CVLR template crate compiles.

- `certoraSolanaProver /Users/loser/projects/sour/target/deploy/sour.so --compilation_steps_only --short_output --msg sour-verification-compile-smoke`
  - Result: exit 0.
  - Confirms the installed Solana prover accepts the built Sour `.so` in local compile-only mode.
  - Full Certora rules are not yet wired into Sour's program build or a `.conf`; that is part of the implementation plan below.

Repeated Cargo warning seen:

- `failed to auto-clean cache data ... num-bigint-0.4.6/Cargo.toml ... Permission denied`
- This warning did not block builds, tests, Kani, Lean, or CVLR cargo-check.

## Confirmed P0 Breakages

### P0-A: Curve LP Capacity Under-counts Fractional qmax

Observed source shape:

- `programs/sour-math/src/margin.rs:216` computes:
  - `qmax_underlying = qmax_storage / FIXED_ONE`
  - returns `0` when `qmax_storage < FIXED_ONE`
- This loses legitimate fractional exposure introduced by the v0.4 `qmax_storage = qmax_underlying * FIXED_ONE` convention.

Concrete counterexample:

- `qmax_storage = 2_147_483_648` (`FIXED_ONE / 2`)
- `entry = 100_000_000`
- `p_hi = 110_000_000`
- current helper returns `0`
- exact formula returns `(qmax_storage * (p_hi - entry)) / FIXED_ONE = 5_000_000`

Security impact:

- LP capacity checks can accept many small CURVE positions whose aggregate fractional max payout is nonzero.
- This undermines `CurveOiCapacityExceeded` as a backing check.

Required fix:

- Change `curve_max_lp_loss_per_position_micros` to multiply before dividing:
  - long: `(qmax_storage as u128 * max(0, p_hi - entry)) / FIXED_ONE`
  - short: `(qmax_storage as u128 * max(0, entry - p_lo)) / FIXED_ONE`
- Preserve checked arithmetic and `u128` result.
- Update comments that currently say `qmax_underlying = qmax_storage >> 32` and "whole base units."

Files likely owned by this fix:

- `programs/sour-math/src/margin.rs`
- `programs/sour/src/instructions/upsert_position.rs`
- `programs/sour/src/instructions/close_position.rs`
- `programs/sour/src/instructions/clear_batch.rs`
- `packages/sour-sdk/src/decoders.ts`
- `packages/sour-sdk/src/bracket.ts`
- Existing unit tests under `programs/sour/src/tests/*` and `programs/sour-math/src/*` tests.

Acceptance tests:

- Add a Sour math unit test for the concrete `5_000_000` counterexample.
- Replace/update any current test expecting sub-`FIXED_ONE` qmax max-loss to be zero.
- Existing `sour-verification/tests/sour_breakdown_cases.rs` case should become a passing regression after the verification repo is repointed to fixed Sour.
- Update Kani expected-failure harness into a green proof or remove the expected-failure gate.

### P0-B: Positive Close PnL Can Mint Trader Collateral

Observed source shape:

- `programs/sour/src/instructions/close_position.rs:202-220`
- The handler credits `trader_account.usdc_collateral += pnl_u`.
- Then it debits `sour_vault.total_assets` with `saturating_sub(pnl_u)`.
- If `pnl_u > prior_assets`, trader still receives the full credit while LP debit is capped at `prior_assets`.

Concrete counterexample:

- `total_assets = 1_000_000`
- `final_pnl = 5_000_000`
- trader credit = `5_000_000`
- LP debit = `1_000_000`

Security impact:

- This is a direct conservation break. It is the clearest "can be drained/minted" style issue in the current suite.

Required fix:

- Replace the saturating debit with exact debit semantics.
- Recommended v1 security patch:
  - Compute `pnl_u`.
  - Require an exact backing source before crediting trader.
  - Debit vault/reserve first, then credit trader, or credit after all checked debits succeed.
  - If `bad_debt_reserve` is intended to back winner payouts, debit `total_assets + bad_debt_reserve` exactly with deterministic ordering.
  - If reserve is only for liquidation overflow, require `total_assets >= pnl_u` and return `InsufficientCollateral` or a new `VaultInsolvent` error.
- Do not keep any positive-PnL path where trader credit can exceed vault/reserve debit.

Recommended policy decision:

- Prefer `total_assets` as the payout source for winner PnL and fail close if `total_assets < pnl_u`.
- Keep `bad_debt_reserve` reserved for cascade/insolvency mechanics unless the protocol spec explicitly says it backs winner settlement.
- If failing close creates unacceptable UX, implement a partial settlement design separately; do not silently mint.

Files likely owned by this fix:

- `programs/sour/src/instructions/close_position.rs`
- `programs/sour/src/tests/close_position_tests.rs`
- Verification model: `crates/sour-verifier/src/accounting.rs`
- Verification tests: `crates/sour-verifier/tests/sour_breakdown_cases.rs`
- Kani harness: `crates/sour-verifier/src/kani_harnesses.rs`
- Lean model: `lean/SourVerification/Sour/NoBadDebt.lean`

Acceptance tests:

- Add Sour unit/surfpool test that attempts profitable close with insufficient LP assets and verifies rejection with no state mutation.
- Add passing test that profitable close with sufficient LP assets produces exact conservation:
  - `delta_trader_collateral == -delta_sour_vault_total_assets`
  - `outstanding_winner_pnl` releases only the old active contingent PnL.
- Convert the verification expected failure into a green proof.

## P0 Hardening Fixes

### P0-C: Bind Passed Price Accounts to Market.price_account_N

Observed source shape:

- `state.rs:382-385` stores `price_account_0..3`.
- `state.rs:450-493` validates configured account count and unused zero slots.
- `price/mod.rs:32-49` checks only price account owner.
- `upsert_position.rs:419-421`, `close_position.rs:97-99`, and `clear_batch.rs:262-264` check only owner for the first price account.

Risk:

- For real oracle sources, an attacker may pass a different account owned by the correct oracle program unless the adapter itself fully binds feed identity.
- The market already stores canonical price-account addresses; handlers should enforce them.

Required fix:

- Add helper, probably in `price/mod.rs` or `state.rs`:
  - `validate_price_account_keys(market: &Market, views: &[AccountView], required: usize)`.
  - For each required `i`, require `*views[i].address() == market.price_account_i`.
  - Keep existing owner validation too.
- Call it in:
  - `upsert_position`
  - `close_position`
  - `clear_batch`
- Use source-specific required count already computed in each handler.

Acceptance tests:

- Unit test helper rejects mismatched key even when owner is correct.
- Surfpool/e2e scenario for PythPull market where a wrong Pyth receiver-owned account is passed and instruction rejects.

### P0-D: Authenticate Remaining Position/Trader/Market Accounts in Raw-Byte Handlers

Observed source shape:

- `clear_batch.rs:325-460` partitions tail remaining accounts by discriminator byte.
- `withdraw_collateral.rs:68-127` scans remaining positions by discriminator byte and embedded trader address.
- These paths read raw bytes for CU reasons, but discriminator bytes alone do not establish account ownership/PDA authenticity.

Risk:

- A caller may pass fake or unrelated remaining accounts whose raw bytes look like `Position`, `TraderAccount`, or `Market`.
- Even if Solana write locks and program ownership limit some writes, these handlers should reject non-Sour accounts instead of silently trusting bytes.

Required fix:

- Add low-CU account-view validators:
  - Owner/program-id check for every `Position`, `TraderAccount`, and `Market` tail account.
  - Address/PDA binding for positions:
    - `Position.trader_account` embedded bytes match a real passed `TraderAccount`.
    - `Position.market` embedded bytes equal `self.market.address()` or a passed foreign `Market`.
    - If affordable, validate position PDA seeds `[b"pos", trader_account, market]`.
  - For `TraderAccount`, validate address consistency and owner/program.
  - For foreign `Market`, validate owner/program and reject short buffers.
- Decide whether short/fake tail accounts should be ignored or rejected. Recommendation: reject malformed accounts once they appear after the price prefix, because silent ignore can hide malformed builders and attacks.

Acceptance tests:

- `clear_batch` rejects fake position bytes in an account not owned by Sour.
- `clear_batch` rejects position whose embedded market is not current market and no matching foreign market account is passed.
- `withdraw_collateral` rejects fake position bytes that attempt to satisfy `position_count`.
- Tests must include both benign zeroed positions and malformed fake accounts.

## P1 Accounting and Risk Fixes

### P1-A: Re-upsert Must Either Be Rejected or Delta All Existing Accounting

Observed source shape:

- `upsert_position.rs:216-217` says modify-in-place delta is deferred.
- Current handler mutates OI before it knows the old position will be overwritten.
- It partially handles `position_count` and `cross_im_used`, but does not fully delta all prior:
  - `market.long_oi` / `short_oi`
  - `curve_max_lp_loss_long` / `short`
  - prior isolated `collateral_locked`
  - prior mode/tier/side changes
  - prior active outstanding/winner state if re-upsert on a live position is allowed

Risk:

- Same PDA re-upsert can drift OI, capacity counters, collateral locks, and IM accounting.

Recommended v1 security patch:

- Reject active re-upsert outright:
  - If existing `position.trader_account != Address::default()` and not zeroed, return `PositionsExist` or a clearer new error.
  - Continue allowing:
    - true fresh init
    - reopen after cascade-zeroed position if current design needs that.
- Full delta-based modify-in-place can be a later feature with its own spec and tests.

Acceptance tests:

- Active same-market re-upsert rejects with no OI/collateral/cap mutation.
- Zeroed reopen still increments active counts and does not double-release old `cross_im_contribution`.

### P1-B: Cross IM Gate Must Include Open Fee Debit

Observed source shape:

- `upsert_position.rs:364-377` checks `new_used <= current_collat`.
- Lock debit happens at `383-385`.
- Fee debit happens later at `536-543`.

Risk:

- Cross-mode open can pass IM check using pre-fee collateral, then the fee reduces free collateral below `cross_im_used`.

Required fix:

- Either calculate fee before the cross IM gate or include the maximum/current fee in the gate:
  - `new_used <= current_collat - fee`
- Preserve rollback semantics and no state mutation on rejection.

Acceptance tests:

- Construct a Cross account where `new_cross_im_contribution <= collateral` but `new_cross_im_contribution + fee > collateral`; open must reject.
- Passing case where collateral covers both IM and fee.

### P1-C: Admin Risk Parameter Bounds

Observed source shape:

- `update_market_params.rs:50-70` writes values with no bounds.
- `update_tier_table.rs:62-68` writes tier rows with no bounds.

Risk:

- Admin can accidentally set nonsensical or unsafe values:
  - fees above 100%
  - `maintenance_margin_bps > initial_margin_bps`
  - zero IM/MM on active tiers
  - leverage inconsistent with IM
  - stale/conf/gap thresholds that disable intended protections unintentionally

Required fix:

- Add validation before writes:
  - `fee_micros <= 1_000_000`
  - `0 < maintenance_margin_bps <= initial_margin_bps <= 10_000`
  - `max_leverage_x100 > 0`
  - tier rows satisfy the same IM/MM constraints.
  - optionally require `lev_x100 * im_bps <= 1_000_000` relationship or derive from one canonical source.
- Add explicit errors or reuse `InvalidMarketParams`.

Acceptance tests:

- Each invalid param rejects and leaves market unchanged.
- Valid default tier table still passes.

### P1-D: Position Count Semantics

Observed source shape:

- `withdraw_collateral.rs:32-36` comments say zeroed positions are included in the integrity count.
- `withdraw_collateral.rs:94-97` increments `owned_count` before skipping zeroed exposure.
- `clear_batch` comments and B17 behavior decrement active counts on cascade zero.

Risk:

- Mixed semantics can either block withdrawals after cascade-zero or let fake zeroed accounts satisfy count.

Recommendation:

- Define `TraderAccount.position_count` and `Market.position_count` as active-position counts only.
- `withdraw_collateral` should count active positions only.
- Zeroed-but-not-closed accounts should not contribute to active count and should not be required for withdrawal.
- Update comments and tests to match this definition.

Acceptance tests:

- Cross trader with one cascade-zeroed position and no active positions can withdraw after health check.
- Fake zeroed accounts cannot satisfy active count.
- Position count matches actual active Sour-owned position accounts.

## Verification Follow-Up

### Kani

Current state:

- Green lane passes but skips direct `fee_for_notional` function proof due CBMC timeout.
- Expected-failure lane intentionally expects current Sour breakages.

Next work:

- After fixing Sour, convert expected-failure harnesses into positive proofs/regression proofs.
- Add a pure arithmetic Kani lemma for fee:
  - Avoid the current direct wrapper if CBMC keeps timing out on `u128 / 1_000_000`.
  - Or verify a smaller pure helper that `fee_for_notional` delegates to.
- Keep `scripts/kani.sh` fast and deterministic. Put expensive stress harnesses in a separate script.

### Lean

Current state:

- Lean builds and models the exact counterexamples.

Next work:

- Add theorem for fixed curve formula:
  - For `qmax_storage = FIXED_ONE / 2` and adverse move `10_000_000`, fixed helper equals `5_000_000`.
- Add theorem for exact positive PnL debit:
  - no trader credit without sufficient vault backing.

### Certora/CVLR

Current state:

- `certora/sour-cvlr` compiles as a Rust CVLR template crate.
- `certoraSolanaProver` accepts the built Sour `.so` in compile-only mode.
- Full rule execution is not wired.

Next work:

- Decide between:
  - embedding CVLR rules in the Sour program crate behind a feature, or
  - maintaining a dedicated verification crate with a proper Certora `.conf`/build script that pulls in Sour.
- Add a real `.conf` that points at the built `.so` and selected rules.
- Start with instruction-level rules for:
  - close positive-PnL conservation
  - upsert curve capacity exactness
  - price account key binding
  - withdraw collateral active-count semantics

## Subagent Split for Next Session

Use separate workers with disjoint write ownership. Tell every worker the repo is dirty and they must not revert unrelated changes.

1. Math/cap worker
   - Owns `programs/sour-math/src/margin.rs`, SDK mirror docs/tests, and curve-cap tests.
   - Goal: fix fractional `qmax_storage` capacity accounting.

2. Close/accounting worker
   - Owns `programs/sour/src/instructions/close_position.rs` and close-position tests.
   - Goal: no positive trader credit without exact vault/reserve debit.

3. Oracle/remaining-auth worker
   - Owns price account validation helpers, `upsert_position`, `close_position`, `clear_batch`, `withdraw_collateral` auth checks, and corresponding tests.
   - Goal: bind oracle keys and reject fake remaining accounts.

4. Upsert/admin worker
   - Owns active re-upsert behavior, cross IM fee gate, admin param validation, and related unit tests.
   - Goal: eliminate accounting drift and unsafe admin params.

5. Verification worker
   - Owns `/Users/loser/projects/sour-verification`.
   - Goal: update Rust/Kani/Lean/CVLR suites after Sour fixes land, converting expected failures into green regression proofs.

## Next Session Starting Commands

Run these before editing:

```bash
cd /Users/loser/projects/sour
git status --short

cd /Users/loser/projects/sour-verification
./scripts/test-green.sh
./scripts/test-breakdowns.sh
./scripts/kani.sh
./scripts/kani-expected-failures.sh
(cd lean && lake build)
cargo check --manifest-path certora/sour-cvlr/Cargo.toml
```

Expected before fixes:

- Green Rust/Kani/Lean/CVLR lanes pass.
- Breakdown and expected-failure lanes demonstrate the two P0 bugs.

Expected after fixes:

- Sour-side tests for the bugs pass.
- `sour-verification` no longer treats the two P0 bugs as expected failures.
- Any old "expected failure" script is renamed or rewritten so fixed behavior is a required success.

## Completion Criteria

The next implementation session is complete only when:

- P0-A and P0-B are fixed in Sour.
- Oracle key binding is enforced in every price-reading handler.
- Fake remaining accounts are rejected where raw-byte handlers scan positions/traders/markets.
- Active re-upsert behavior is explicitly rejected or fully delta-accounted.
- Cross IM gate accounts for fees.
- Admin risk params reject unsafe values.
- Tests cover every changed behavior.
- Verification repo is updated so:
  - Rust green tests pass.
  - Breakdown tests are converted to passing regressions or removed from the failure lane.
  - Kani green lane passes.
  - Kani expected-failure lane no longer expects fixed bugs.
  - Lean builds.
  - CVLR crate checks and Certora compile-only smoke still pass.

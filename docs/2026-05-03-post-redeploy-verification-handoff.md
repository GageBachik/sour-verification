# Sour Post-Redeploy Verification Handoff

Date: 2026-05-03

Repos:
- Sour program repo: `/Users/loser/projects/sour`
- Verification repo: `/Users/loser/projects/sour-verification`

Purpose: preserve the end-of-session state so another agent can resume without relying on chat history.

## Current State

The Sour hardening fixes have been implemented, rebuilt, and redeployed to devnet. The deployed devnet program bytes were checked against the freshly built local SBF artifact and matched exactly.

Program:
- Program id: `souryQgnM1xiNuGcmVYLPGT3MKqnGN8QTqP8zk8eape`
- ProgramData: `7apPBrrphVwHAx2TmvZRaPPKmtGGgYcwA6Q4n24DzPkT`
- Upgrade authority: `GKPoVMxhDTi3SGxK8wRgguwfXjV8EmNkpqCfeQWyhniT`
- Last deployed slot observed: `459857325`
- Local/deployed SBF size: `229728`
- Local/deployed SBF SHA-256: `9ef555dcfd98bf32c4b4e4ba5677761d89f56b1d17dc283810392bcbb59302be`

Important workspace note:
- `/Users/loser/projects/sour` is a broad dirty worktree with many unrelated modified/untracked files from prior work. Do not revert unknown changes.
- `/Users/loser/projects/sour-verification` is an uncommitted/untracked verification workspace.
- No Kani/CBMC/cargo-kani verifier processes were left running at the end of this session.

## Fixes Landed

P0-A: fractional CURVE LP capacity accounting
- Main file: `/Users/loser/projects/sour/programs/sour-math/src/margin.rs`
- `curve_max_lp_loss_per_position_micros` now computes capacity from `qmax_storage` directly:
  - `(qmax_storage as u128 * max_adverse) / FIXED_ONE`
- Regressions cover long and short `qmax_storage = FIXED_ONE / 2`, expected loss `5_000_000`.
- SDK comments/types were updated where they still described whole-unit-only qmax semantics.

P0-B: positive close PnL conservation
- Main file: `/Users/loser/projects/sour/programs/sour/src/instructions/close_position.rs`
- Positive final PnL now requires sufficient `sour_vault.total_assets`, debits LP assets exactly, then credits trader collateral.
- Underfunded positive PnL now rejects instead of saturating.
- Tests cover underfunded rejection and fully backed exact debit.

P0-C: price account binding
- Main file: `/Users/loser/projects/sour/programs/sour/src/price/mod.rs`
- Added helpers to compare required passed price account prefix against configured `Market.price_account_0..3`.
- Wired into:
  - `upsert_position`
  - `close_position`
  - `clear_batch`
- Owner/source validation remains in place.
- Deployed devnet smoke confirmed good SOL Pyth account succeeds and substituted BTC Pyth account rejects.

P0-D: raw remaining account authentication and active counts
- Main files:
  - `/Users/loser/projects/sour/programs/sour/src/instructions/clear_batch.rs`
  - `/Users/loser/projects/sour/programs/sour/src/instructions/withdraw_collateral.rs`
- Sour discriminator tail accounts after price prefix must be Sour-owned and full-length.
- Raw positions must bind to the embedded trader account; foreign market positions require the matching foreign market account.
- `withdraw_collateral` counts active positions only; zeroed or `qmax == 0` positions are skipped before count/snapshot construction.

P1: upsert/admin hardening
- Main files:
  - `/Users/loser/projects/sour/programs/sour/src/instructions/upsert_position.rs`
  - `/Users/loser/projects/sour/programs/sour/src/instructions/update_market_params.rs`
  - `/Users/loser/projects/sour/programs/sour/src/instructions/update_tier_table.rs`
  - `/Users/loser/projects/sour/programs/sour-math/src/errors.rs`
- Active same-PDA re-upsert rejects early with `PositionsExist`.
- True fresh init and zeroed reopen remain allowed.
- Cross-margin IM gate uses collateral after open fee.
- Admin market params and tier rows are validated before writes.
- Appended `InvalidMarketParams` after existing error codes.

Verification mirror updates:
- `/Users/loser/projects/sour-verification/crates/sour-verifier/src/accounting.rs`
- `/Users/loser/projects/sour-verification/crates/sour-verifier/src/kani_harnesses.rs`
- `/Users/loser/projects/sour-verification/crates/sour-verifier/tests/sour_breakdown_cases.rs`
- `/Users/loser/projects/sour-verification/scripts/kani.sh`
- `/Users/loser/projects/sour-verification/scripts/kani-expected-failures.sh`
- `/Users/loser/projects/sour-verification/lean/SourVerification/Sour/NoBadDebt.lean`

The old expected-failure lane is retired; fixed P0 cases are now required green obligations.

## Verification Evidence

Native contract/math:
- `cargo test -p sour --lib`
  - Passed: `201/201`
- `cargo test -p sour-math --lib`
  - Passed: `43/43`
- `pnpm --filter @sour/sdk test -- bracket.test.ts`
  - Passed: `16/16`
- `pnpm --filter @sour/sdk typecheck`
  - Passed
- `cargo build-sbf --manifest-path programs/sour/Cargo.toml --features devnet`
  - Passed

Formal/verification repo:
- `./scripts/test-green.sh`
  - Passed: `15/15`
- `./scripts/test-breakdowns.sh`
  - Passed: `4/4`
- `cd lean && lake build`
  - Passed
- `cargo check --manifest-path certora/sour-cvlr/Cargo.toml`
  - Passed
- `cargo fmt -p sour-verifier --check`
  - Passed
- `./scripts/kani.sh`
  - Passed
  - Includes fractional CURVE capacity obligations and exact positive close-PnL debit obligation.
  - Still skips direct `fee_for_notional` Sour-boundary proof due CBMC timeout; this remains proof-engineering debt documented in `docs/fix-implementation-spec.md`.
- `certoraSolanaProver /Users/loser/projects/sour/target/deploy/sour.so --compilation_steps_only --short_output --msg sour-verification-compile-smoke`
  - Passed

Hygiene:
- `git diff --check` in `/Users/loser/projects/sour`
  - Passed
- `git diff --check` in `/Users/loser/projects/sour-verification`
  - Passed

Known non-blocking warning:
- Cargo still emits a global cache auto-clean warning for `num-bigint-0.4.6/Cargo.toml` permission denied. It did not block tests/builds.

## On-Chain Devnet Evidence

Artifact match:
- Command path used:
  - Dumped devnet program to `/tmp/sour-devnet-dump.so`
  - Compared `target/deploy/sour.so` and dumped bytes
- Result:
  - Same size: `229728`
  - Same SHA-256: `9ef555dcfd98bf32c4b4e4ba5677761d89f56b1d17dc283810392bcbb59302be`

State checks:
- Protocol PDA present and owned by Sour program.
- SourVault PDA present and owned by Sour program.
- USDC vault ATA present and initialized.
- Six launch markets from `.devnet/state/setup.json` are present and owned by Sour program.
- USDC vault token balance observed: `139` devnet USDC.

Devnet smoke:
- Sent `clear_batch` for `sol-usd` using the configured SOL Pyth account.
- Simulation result: success.
- Confirmed tx:
  - `5yfewGJkoEZJcpWQiP5G55Yr3aDPKyQuhcjDk7oRgSyujtoWq86VJJqeExydyykS1bkYxzBCF3MuRjwfeJqouYqB`

Oracle mismatch negative smoke:
- Simulated `clear_batch` for `sol-usd` while substituting BTC's Pyth price account.
- Rejected with `Custom:10`.
- `Custom:10` maps to `InvalidPriceAccounts` in `programs/sour-math/src/errors.rs`.
- This confirms the deployed binary enforces the new configured price-account prefix binding.

## Operational Notes / Concerns

The devnet service pid files are stale:
- `.devnet/run/keeper.pid`
- `.devnet/run/mm.pid`
- `.devnet/run/pyth-pusher.pid`

Observed at handoff:
- Keeper process: not running.
- MM process: not running.
- Pyth pusher process: not running.

Old logs:
- MM stderr shows repeated `insufficient account keys for instruction` errors. These logs appear old and likely predate the updated account-list assumptions around price accounts. Restart MM/keeper from the updated code and watch for recurrence.
- Pyth pusher stderr shows repeated `429 Too Many Requests`. Use a better/quota-backed endpoint or throttle if restarting it.

Live scenario runner caveat:
- `tests/surfpool` live harness is Surfpool-oriented and uses `surfnet_setAccount` plus test-USDC mint-authority assumptions.
- Do not treat `SOUR_RPC_URL=<devnet> SOUR_HARNESS_MODE=live tests/surfpool/runner.ts all` as a safe devnet regression runner without adapting the setup path.
- For devnet, prefer targeted transaction smoke scripts or write a devnet-safe harness that uses real existing mints/accounts and does not rely on Surfpool cheatcodes.

## Suggested Next Steps

1. Restart keeper/MM/pyth-pusher from updated code and watch logs for the old `insufficient account keys` and `429` issues.
2. Add a devnet-safe regression harness for the hardened surfaces:
   - configured price account accepted
   - mismatched price account rejected
   - clear/close/upsert with required Pyth remaining accounts
   - withdraw with active-only position counting
3. If preparing for mainnet, repeat artifact hash comparison against the intended mainnet deployment and confirm upgrade authority custody/rollback plan.
4. Leave the formal verification matrix as a required pre-upgrade gate:
   - Sour lib tests
   - sour-math lib tests
   - SDK test/typecheck
   - SBF build
   - verification green/breakdown
   - Kani
   - Lean
   - CVLR
   - Certora compile smoke


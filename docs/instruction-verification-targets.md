# Sour Instruction Verification Targets

These properties require Solana instruction-level verification or e2e account
fixtures. They are documented here so the Rust/Kani pure-math lane does not
pretend to cover runtime account authenticity.

## Conservation

- `usdc_vault.amount == sour_vault.total_assets + sour_vault.bad_debt_reserve + Σ trader.usdc_collateral + Σ active position.collateral_locked`.
- `SourVault.outstanding_winner_pnl == Σ max(position.realized_pnl, 0)` over active positions.
- `sour_mint.supply == sour_vault.total_shares`.
- Positive trader credits must be matched by LP/reserve debits; saturated arithmetic must not create unpaid collateral.
- v0.5.1-P1c aggregate-budget conservation: `Protocol.aggregate_max_lp_loss == Σ market.worst_case_lp_loss(max_mm_bps, market.price_smoothed)` across all markets. Hot-path counter is mutated at exactly three sites — `upsert_position`, `close_position`, `clear_batch` cascade-zero — each via `Protocol::update_aggregate(delta)`. Drift recovery: permissionless `recompute_aggregate` (disc=23) re-derives the value from the authoritative per-market state. Pre-v0.5.1-P1c Protocol accounts decode the field as 0 (zero-init reserved slot); operator MUST call `recompute_aggregate` once post-deploy to sync. Until then the counter under-counts and the cap is effectively wider — strictly safer (no false rejects) for the upgrade window.

## Index Exactness

- `market.long_oi == Σ active long qmax`.
- `market.short_oi == Σ active short qmax`.
- `market.position_count == count(active positions on market)`.
- `trader.position_count == count(active positions for trader)`.
- `trader.cross_im_used == Σ active cross_im_contribution`.

## Account Authenticity

- `clear_batch` remaining `Position`, `TraderAccount`, and foreign `Market` accounts must be program-owned and PDA-valid, not merely byte buffers with matching discriminators.
- `withdraw_collateral` cross-margin position remaining accounts must be the real active positions for the trader.
- Price remaining accounts must match `market.price_account_0..N`; owner checks alone permit valid-oracle substitution.
- v0.5.1-P1c `recompute_aggregate` (disc=23) remaining `Market` accounts MUST pass program-owner check + PDA re-derivation against the embedded `Market.asset_id` (`recompute_aggregate.rs:71-91`). Without re-derivation, a spoofed account that shares the discriminator + owner via reallocation tricks could inflate the running counter and let a real upsert bypass the cap.

## Admin Bounds

- Market parameters should preserve nonzero IM/MM, `mm_bps <= im_bps`, bounded fees, sane confidence/staleness caps, nonpathological trigger/pause values, and leverage tiers consistent with margin requirements.
- v0.5.1-P1c `aggregate_budget_bps` (admin-tunable via `admin_update_risk_budget`) bounded so `aggregate_cap = total_assets × aggregate_budget_bps / max_mm_bps` does not wrap u128. Default 10_000 (100% — loose for soft launch). Pre-v0.5.0-E Protocol accounts decode 0; the on-chain handler treats 0 as "use default 10_000" — operator MUST be aware that the `0` byte slot maps to a 100% budget, not "block all opens".

## v0.5.1-P1c Aggregate-Cap Surface (NEW)

Per-instruction enforcement coverage:

- **`upsert_position` (existing ix)** — at line 648 of `programs/sour/src/instructions/upsert_position.rs`, BEFORE the per-user cap, the handler computes the proposed post-trade `Market::worst_case_lp_loss`, derives the signed delta vs the pre-trade snapshot, then `require!(proposed_aggregate >= 0 && proposed <= aggregate_cap, AggregateBudgetExceeded)`. On Ok, the actual delta is applied via `protocol.update_aggregate(actual_delta)` after the CURVE-mode mutations land. On Err, runtime rollback unwinds OI / fee / lock mutations made above.
- **`close_position` (existing ix)** — symmetric decrement via `update_aggregate(-removed)`. Underflow guard surfaces as `AggregateBudgetUnderflow` (Custom:43) — recoverable via `recompute_aggregate`.
- **`clear_batch` cascade-zero** — symmetric decrement on cascade-zeroed positions, same underflow handling.
- **`recompute_aggregate` (NEW ix, disc=23, permissionless)** — re-derives `Protocol.aggregate_max_lp_loss = Σ market.worst_case_lp_loss(max_mm_bps, market.price_smoothed)` from authoritative per-market state. Anyone may call (no admin gate). Pass every Market PDA in `remaining_accounts`; markets are authenticated by program-owner check + PDA re-derivation. CU ≈ 150 + 100×market_count (≈ 1.4K for 12 markets, ≈ 10K for 100 — trivial vs the 250K keeper budget).

### Protocol PDA RW expansion (v0.5.1-P1c)

`Protocol` was previously read-only on the trade hot path. v0.5.1-P1c expands it to RW for `upsert_position`, `close_position`, and `clear_batch` (each now mutates `aggregate_max_lp_loss`). Contention impact: every trade ix on every market now serializes through the same Protocol PDA. For the v1 launch market set (≤ 12 markets) this is acceptable; if/when the protocol scales past that, consider migrating `aggregate_max_lp_loss` to a per-market "shadow" account summed lazily by the keeper. (Out of scope for v0.5.1.)

### Formula deviation from spec Part 4

The locked Part 4 spec writes the per-market perp side as `|long_oi - short_oi| × max_mm_bps / 10_000`, assuming OI is denominated in µUSDC. The codebase stores `Market.long_oi`/`short_oi` in `qmax_storage` units (FIXED_ONE-scaled base units; see v0.4 B1 storage scaling). To produce a result in µUSDC — required so it sums dimensionally with `curve_side` — `Market::worst_case_lp_loss` (`state.rs:547`) actually computes:

```text
perp_side = |long_oi - short_oi| × mark_micros × max_mm_bps / 10_000 / FIXED_ONE
```

This deviation is documented inline at the function and mirrored byte-for-byte in `crates/sour-verifier/src/aggregate.rs::MarketModel::worst_case_lp_loss` so the Kani / Lean proofs certify the as-built code rather than the as-written spec. The relationship `perp_side_µUSDC = perp_side_qmax × mark / FIXED_ONE` is dimensionally exact; the byte-exact mirror in `clear_batch::market_worst_case_lp_loss_bytes` reads `price_smoothed` from the same offset the typed handler uses, so all four call sites (typed handler, byte helper, recompute, verification model) stay in lockstep. Spec Part 4 should be amended to reference this convention.

### Verification status

- Kani `proof_aggregate_cap_invariant_holds_under_update` — system invariant `aggregate_pre ≤ cap` is preserved by `update_aggregate(delta)` when the upsert check passed. Verified.
- Kani `proof_recompute_matches_per_market_sum_n4` — `recompute_aggregate` byte-equals `Σ Market::worst_case_lp_loss` over four bounded-symbolic markets. Verified.
- Kani `proof_update_aggregate_no_underflow` — `update_aggregate` returns `Err(Underflow)` rather than wrapping when `prior + delta < 0`; counter unchanged on Err. Verified.
- Lean `aggregate_bound_general_n` — pure-Nat, general-N decomposition. No `sorry`, no Mathlib.
- Lean `update_aggregate_preserves_bound` — cap-preservation under the upsert enforcement check. No `sorry`.

## Concrete Failure Candidates

- Fake remaining position in `clear_batch` can enter solver inputs and outstanding-liability deltas.
- Fake/zeroed remaining position in `withdraw_collateral` can satisfy `owned_count == position_count` while hiding real exposure.
- Oracle substitution can clear/close/upsert against the wrong valid price account.
- Cross-position open fee can reduce collateral after the IM check, making `cross_im_used > usdc_collateral`.
- Active isolated re-upsert can overwrite `collateral_locked` after debiting a new lock without releasing the old one.
- Active re-upsert can add OI/cap twice while close subtracts one final qmax.
- Positive close PnL can credit more trader collateral than LP assets debited because of `saturating_sub`.
- CURVE tiny qmax can bypass the LP cap because current cap floors to whole base units before multiplying by price risk.
- v0.5.1-P1c: spoofed `Market` account passed to `recompute_aggregate` (matching disc + owner via reallocation) could inflate the counter beyond the true sum; the PDA re-derivation guard in `recompute_aggregate.rs:79-91` is the sole defense — must hold or the aggregate cap is bypassable.
- v0.5.1-P1c: drift between hot-path counter and true `Σ worst_case_lp_loss` is recoverable via `recompute_aggregate` but undetectable on-chain without an off-chain monitor; the keeper SHOULD periodically diff the counter against `Σ` and call recompute on divergence.


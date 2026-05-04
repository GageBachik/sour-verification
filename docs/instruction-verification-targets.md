# Sour Instruction Verification Targets

These properties require Solana instruction-level verification or e2e account
fixtures. They are documented here so the Rust/Kani pure-math lane does not
pretend to cover runtime account authenticity.

## Conservation

- `usdc_vault.amount == sour_vault.total_assets + sour_vault.bad_debt_reserve + Σ trader.usdc_collateral + Σ active position.collateral_locked`.
- `SourVault.outstanding_winner_pnl == Σ max(position.realized_pnl, 0)` over active positions.
- `sour_mint.supply == sour_vault.total_shares`.
- Positive trader credits must be matched by LP/reserve debits; saturated arithmetic must not create unpaid collateral.

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

## Admin Bounds

- Market parameters should preserve nonzero IM/MM, `mm_bps <= im_bps`, bounded fees, sane confidence/staleness caps, nonpathological trigger/pause values, and leverage tiers consistent with margin requirements.

## Concrete Failure Candidates

- Fake remaining position in `clear_batch` can enter solver inputs and outstanding-liability deltas.
- Fake/zeroed remaining position in `withdraw_collateral` can satisfy `owned_count == position_count` while hiding real exposure.
- Oracle substitution can clear/close/upsert against the wrong valid price account.
- Cross-position open fee can reduce collateral after the IM check, making `cross_im_used > usdc_collateral`.
- Active isolated re-upsert can overwrite `collateral_locked` after debiting a new lock without releasing the old one.
- Active re-upsert can add OI/cap twice while close subtracts one final qmax.
- Positive close PnL can credit more trader collateral than LP assets debited because of `saturating_sub`.
- CURVE tiny qmax can bypass the LP cap because current cap floors to whole base units before multiplying by price risk.


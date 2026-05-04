# Sour Verification Design

**Goal:** Build a standalone verification workspace that tests and proves Sour's no-drain invariants from day one.

**Scope:** This repo verifies the Sour repo at `/Users/loser/projects/sour`; “flow perps” refers only to Sour, never Tide Pool.

## Architecture

The workspace has three verification layers.

1. Rust executable model: property tests and deterministic counterexamples over `sour-math` plus a small scalar mirror of vault accounting.
2. Kani proof harnesses: bounded symbolic proofs for pure Rust invariants that should hold for all inputs in constrained domains.
3. Lean/Certora artifacts: Lean captures economic no-bad-debt theorems; CVLR templates capture the intended Certora Solana rules for instruction-level verification.

## Verified Properties

- Curve fill ratios stay in `[0, 1]` and are monotone.
- Fees cannot exceed notional when fee rate is at most 100%.
- Epsilon is always capped by `max_epsilon_bps`.
- Winner-PnL accounting equals `max(new, 0) - max(old, 0)`.
- LP shares redeem against available assets, not inflated total assets.
- Cross-margin withdrawal fails when post-withdraw health is below maintenance.
- Cascade zeroes worst losing positions first until remaining health is sufficient.
- The verification suite exposes known mismatches as counterexamples instead of hiding them.

## Known Counterexample

Sour's current CURVE LP-cap helper undercounts sub-unit risk:

```text
current = floor(qmax_storage / 2^32) * adverse_distance
exact   = floor(qmax_storage * adverse_distance / 2^32)
```

For `qmax_storage = 0.5 * 2^32` and a 10 USDC adverse move, current returns `0`; exact returns `5_000_000` micro-USDC.

## Tooling Assumptions

Kani, Lean, and Certora are optional local tools. This machine currently has Rust/Cargo, but not Kani, Lean/Lake, or Certora. The repo therefore makes Rust/property tests executable immediately and stores Kani/Lean/Certora artifacts so they can be run once the tools are installed.


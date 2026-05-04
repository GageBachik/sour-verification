# Sour CVLR Rule Templates

Certora's Solana workflow uses Rust-embedded CVLR rules. The official docs
describe `#[rule]`, `nondet`, `cvlr_assume!`, and `cvlr_assert!`, and the
`certoraSolanaProver --rule <rule_name>` entrypoint.

This directory is a template crate for Sour's pure-rule layer. The rules here
target `sour-math` first because those functions are Solana-runtime-free. Once
instruction-level CVLR is wired into the Sour program crate, mirror these rules
around the actual handlers.

Run shape once Certora is installed and configured:

```bash
certoraSolanaProver --rule rule_fee_never_exceeds_notional
```

For full Solana handler verification, add Sour-specific inlining and summary
files and run with:

```bash
certoraSolanaProver --solana_inlining path/to/inlining.txt \
  --solana_summaries path/to/summaries.txt \
  --rule rule_withdraw_collateral_keeps_cross_margin_healthy
```


# Sour Verification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create a standalone verification repo for Sour no-drain and accounting invariants.

**Architecture:** Use a Rust crate for executable property tests and Kani harnesses, a Lean project for economic theorems, and a Certora CVLR template crate for instruction-level rule authoring.

**Tech Stack:** Rust, `proptest`, Kani, Lean 4, Certora CVLR.

---

### Task 1: Rust Verification Crate

**Files:**
- Create: `Cargo.toml`
- Create: `crates/sour-verifier/Cargo.toml`
- Create: `crates/sour-verifier/src/lib.rs`
- Create: `crates/sour-verifier/src/accounting.rs`
- Create: `crates/sour-verifier/src/exact.rs`
- Test: `crates/sour-verifier/tests/accounting_invariants.rs`

- [x] Write failing tests first against the intended accounting and exact-risk APIs.
- [x] Run the targeted test and confirm missing modules fail compilation.
- [x] Implement the scalar accounting and exact fractional LP-loss helpers.
- [x] Run the targeted test and confirm it passes.

### Task 2: Counterexample Lane

**Files:**
- Create: `crates/sour-verifier/tests/sour_breakdown_cases.rs`

- [x] Write a test comparing Sour's current CURVE LP-loss cap to exact fractional math.
- [x] Run it and confirm it fails with `current = 0`, `exact = 5_000_000`.

### Task 3: Kani Harnesses

**Files:**
- Modify: `crates/sour-verifier/Cargo.toml`
- Modify: `crates/sour-verifier/src/lib.rs`
- Create: `crates/sour-verifier/src/kani_harnesses.rs`

- [x] Add `cfg(kani)` harnesses for bounded curve, fee, epsilon, LP withdrawal, and empty health-gate properties.
- [x] Add an `expected-failures` feature for the Sour CURVE LP-loss counterexample harness.

### Task 4: Lean and Certora Artifacts

**Files:**
- Create: `lean/lakefile.lean`
- Create: `lean/lean-toolchain`
- Create: `lean/SourVerification/Sour/NoBadDebt.lean`
- Create: `certora/sour-cvlr/README.md`
- Create: `certora/sour-cvlr/Cargo.toml`
- Create: `certora/sour-cvlr/src/lib.rs`

- [x] Add Lean no-bad-debt and fractional-cap theorems.
- [x] Add CVLR rule templates for fees, epsilon, and curve-cap safety.

### Task 5: Verification Scripts and Results

**Files:**
- Create: `scripts/test-green.sh`
- Create: `scripts/test-breakdowns.sh`
- Create: `scripts/kani.sh`
- Create: `scripts/kani-expected-failures.sh`
- Modify: `README.md`

- [x] Run the green invariant lane.
- [x] Run the breakdown lane and record the failing counterexamples.
- [x] Attempt Kani/Lean/Certora commands and record missing-tool blockers.

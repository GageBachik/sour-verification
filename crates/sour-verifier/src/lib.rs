//! Verification models and proof harness helpers for Sour.
//!
//! This crate deliberately stays Solana-runtime-free. It depends on
//! `sour-math` for the canonical pure functions and adds small executable
//! models for protocol accounting invariants that are otherwise spread
//! across handlers.

pub mod accounting;
pub mod aggregate;
pub mod exact;
pub mod max_oi;

#[cfg(kani)]
mod kani_harnesses;

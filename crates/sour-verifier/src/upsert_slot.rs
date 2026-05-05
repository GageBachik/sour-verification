//! v0.6.0 upsert-slot classification model.
//!
//! Mirrors `programs/sour/src/instructions/upsert_position.rs:51-68`:
//!
//! ```text
//! pub(crate) enum UpsertSlotKind { FreshInit, ZeroedReopen }
//!
//! pub(crate) fn classify_upsert_position_slot(
//!     prior_trader_account: Address,
//!     prior_sign_status: u8,
//! ) -> Result<UpsertSlotKind, ProgramError> {
//!     if prior_trader_account == Address::default() {
//!         return Ok(UpsertSlotKind::FreshInit);
//!     }
//!     if is_zeroed(prior_sign_status) {
//!         return Ok(UpsertSlotKind::ZeroedReopen);
//!     }
//!     Err(SourError::PositionsExist.into())
//! }
//! ```
//!
//! The verifier crate stays Solana-runtime-free. We model `Address` as
//! `[u8; 32]` (the byte-equivalent layout) and replace the `ProgramError`
//! return with a 3-variant `Result` so the proof goal is byte-exact and
//! the error path is explicit. `is_zeroed` is mirrored from
//! `state.rs:111`:
//!
//! ```text
//!   const STATUS_BIT: u8 = 0b0000_0010;       // bit 1
//!   pub fn is_zeroed(s: u8) -> bool { (s & STATUS_BIT) != 0 }
//! ```

/// Status bit in the packed `sign_status` byte. Mirror of
/// `programs/sour/src/state.rs::STATUS_BIT`.
pub const STATUS_BIT: u8 = 0b0000_0010;

/// Bit-test mirror of `state.rs::is_zeroed`.
#[inline(always)]
pub fn is_zeroed(sign_status: u8) -> bool {
    (sign_status & STATUS_BIT) != 0
}

/// Three-variant classification result. The on-chain enum is two-variant
/// + `ProgramError` Err arm; we make every outcome a constructor so the
/// totality proof can pattern-match without coupling to `ProgramError`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpsertSlotKind {
    /// `prior_trader_account == Address::default()` (all-zero pubkey).
    /// On-chain: `Ok(UpsertSlotKind::FreshInit)`.
    FreshInit,
    /// Slot is owned but its packed `sign_status` byte has the `STATUS_BIT`
    /// (zeroed) flag set. On-chain: `Ok(UpsertSlotKind::ZeroedReopen)`.
    ZeroedReopen,
    /// Owned slot whose `sign_status` is still active. On-chain:
    /// `Err(SourError::PositionsExist.into())`.
    PositionsExist,
}

/// Byte-exact mirror of `classify_upsert_position_slot`. The control flow
/// shape is preserved one-to-one — first the address-zero check, then the
/// status-bit check, then the rejection arm.
#[inline]
pub fn classify_upsert_position_slot(
    prior_trader_account: [u8; 32],
    prior_sign_status: u8,
) -> UpsertSlotKind {
    if prior_trader_account == [0u8; 32] {
        return UpsertSlotKind::FreshInit;
    }
    if is_zeroed(prior_sign_status) {
        return UpsertSlotKind::ZeroedReopen;
    }
    UpsertSlotKind::PositionsExist
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_init_when_address_zero() {
        // sign_status with STATUS_BIT set is irrelevant when address is zero —
        // the on-chain function returns FreshInit before checking the byte.
        assert_eq!(
            classify_upsert_position_slot([0u8; 32], 0),
            UpsertSlotKind::FreshInit
        );
        assert_eq!(
            classify_upsert_position_slot([0u8; 32], STATUS_BIT),
            UpsertSlotKind::FreshInit
        );
        assert_eq!(
            classify_upsert_position_slot([0u8; 32], 0xFF),
            UpsertSlotKind::FreshInit
        );
    }

    #[test]
    fn zeroed_reopen_when_address_set_and_status_bit_set() {
        let mut addr = [0u8; 32];
        addr[0] = 1;
        assert_eq!(
            classify_upsert_position_slot(addr, STATUS_BIT),
            UpsertSlotKind::ZeroedReopen
        );
        // Other bits set alongside STATUS_BIT still classify as ZeroedReopen.
        assert_eq!(
            classify_upsert_position_slot(addr, 0xFF),
            UpsertSlotKind::ZeroedReopen
        );
    }

    #[test]
    fn positions_exist_when_owned_and_active() {
        let mut addr = [0u8; 32];
        addr[31] = 0xAA;
        // sign_status = 0 → not zeroed → PositionsExist
        assert_eq!(
            classify_upsert_position_slot(addr, 0),
            UpsertSlotKind::PositionsExist
        );
        // sign_status = SIGN_BIT only (bit 0) → not zeroed → PositionsExist
        assert_eq!(
            classify_upsert_position_slot(addr, 0b0000_0001),
            UpsertSlotKind::PositionsExist
        );
    }

    #[test]
    fn is_zeroed_bit_test_matches_on_chain() {
        // Bit 1 (STATUS_BIT) is the one we test; other bits irrelevant.
        assert!(!is_zeroed(0));
        assert!(!is_zeroed(0b0000_0001));
        assert!(is_zeroed(0b0000_0010));
        assert!(is_zeroed(0b0000_0011));
        assert!(is_zeroed(0xFF));
    }
}

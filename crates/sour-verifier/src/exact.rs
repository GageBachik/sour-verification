use solana_program_error::ProgramError;

const FIXED_ONE: u128 = 1u128 << 32;

/// Fractional version of Sour's CURVE max-LP-loss helper.
///
/// This keeps the `qmax_storage` fractional component until after multiplying
/// by adverse price distance:
///
/// `qmax_storage * adverse_price_distance / FIXED_ONE`
///
/// Sour's current helper first floors `qmax_storage / FIXED_ONE`, which makes
/// sub-base-unit positions reserve zero capacity. This exact helper is the
/// verification oracle for that edge case.
pub fn curve_max_lp_loss_fractional_micros(
    qmax_storage: u64,
    entry_micros: u64,
    p_lo_micros: u64,
    p_hi_micros: u64,
    is_short: bool,
) -> Result<u128, ProgramError> {
    if p_lo_micros >= p_hi_micros || qmax_storage == 0 {
        return Ok(0);
    }

    let entry = entry_micros as u128;
    let p_lo = p_lo_micros as u128;
    let p_hi = p_hi_micros as u128;
    let adverse = if is_short {
        entry.saturating_sub(p_lo)
    } else {
        p_hi.saturating_sub(entry)
    };

    if adverse == 0 {
        return Ok(0);
    }

    (qmax_storage as u128)
        .checked_mul(adverse)
        .map(|v| v / FIXED_ONE)
        .ok_or(ProgramError::ArithmeticOverflow)
}

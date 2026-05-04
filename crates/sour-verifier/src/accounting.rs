use solana_program_error::ProgramError;

/// Scalar mirror of `SourVault` accounting fields needed for verification.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct VaultModel {
    pub total_assets: u64,
    pub total_shares: u64,
    pub outstanding_winner_pnl: u64,
    pub bad_debt_reserve: u64,
}

impl VaultModel {
    #[inline]
    pub fn available_assets(&self) -> u64 {
        self.total_assets
            .saturating_sub(self.outstanding_winner_pnl)
    }

    #[inline]
    pub fn solvency_buffer(&self) -> u128 {
        (self.total_assets as u128) + (self.bad_debt_reserve as u128)
    }
}

pub fn shares_for_deposit(vault: &VaultModel, amount: u64) -> Result<u64, ProgramError> {
    let available = vault.available_assets();
    if vault.total_shares == 0 || available == 0 {
        return Ok(amount);
    }
    let result = (amount as u128)
        .checked_mul(vault.total_shares as u128)
        .and_then(|v| v.checked_div(available as u128))
        .ok_or(ProgramError::ArithmeticOverflow)?;
    u64::try_from(result).map_err(|_| ProgramError::ArithmeticOverflow)
}

pub fn usdc_for_shares(vault: &VaultModel, shares: u64) -> Result<u64, ProgramError> {
    if vault.total_shares == 0 {
        return Err(ProgramError::ArithmeticOverflow);
    }
    let result = (shares as u128)
        .checked_mul(vault.available_assets() as u128)
        .and_then(|v| v.checked_div(vault.total_shares as u128))
        .ok_or(ProgramError::ArithmeticOverflow)?;
    u64::try_from(result).map_err(|_| ProgramError::ArithmeticOverflow)
}

pub fn deposit_lp(vault: &mut VaultModel, amount: u64) -> Result<u64, ProgramError> {
    let shares = shares_for_deposit(vault, amount)?;
    vault.total_assets = vault
        .total_assets
        .checked_add(amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    vault.total_shares = vault
        .total_shares
        .checked_add(shares)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    Ok(shares)
}

pub fn withdraw_lp(vault: &mut VaultModel, shares: u64) -> Result<u64, ProgramError> {
    let usdc_out = usdc_for_shares(vault, shares)?;
    vault.total_assets = vault
        .total_assets
        .checked_sub(usdc_out)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    vault.total_shares = vault
        .total_shares
        .checked_sub(shares)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    Ok(usdc_out)
}

/// Mirror `close_position`'s high-level PnL accounting without Solana CPIs.
///
/// `realized_pnl_old` is the active position contribution being removed from
/// `outstanding_winner_pnl`; `final_pnl` is then applied to LP/trader balances.
pub fn apply_close_pnl(
    vault: &mut VaultModel,
    trader_collateral: u64,
    realized_pnl_old: i64,
    final_pnl: i64,
) -> Result<u64, ProgramError> {
    let next_outstanding = vault
        .outstanding_winner_pnl
        .saturating_sub(realized_pnl_old.max(0) as u64);

    if final_pnl > 0 {
        let pnl = final_pnl as u64;
        if vault.total_assets < pnl {
            return Err(ProgramError::ArithmeticOverflow);
        }
        let next_collateral = trader_collateral
            .checked_add(pnl)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        vault.total_assets -= pnl;
        vault.outstanding_winner_pnl = next_outstanding;
        Ok(next_collateral)
    } else if final_pnl < 0 {
        let loss = (-(final_pnl as i128)) as u64;
        let next_collateral = trader_collateral
            .checked_sub(loss)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        vault.total_assets = vault
            .total_assets
            .checked_add(loss)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        vault.outstanding_winner_pnl = next_outstanding;
        Ok(next_collateral)
    } else {
        vault.outstanding_winner_pnl = next_outstanding;
        Ok(trader_collateral)
    }
}

use anchor_lang::prelude::*;

declare_id!("DiqrzjAcbEyhZVCtwkTTMdHY5G4CDNspkQCEoMHE8VCV");

pub const FP_SCALE: u128 = 1_000_000_000; // 1e9 fixed-point for USD
pub const BPS_DENOM: u128 = 10_000;

#[program]
pub mod tranche_vault {
    use super::*;

    pub fn initialize_vault(
        ctx: Context<InitializeVault>,
        authority: Pubkey,
        senior_apy_cap_bps: u16,
    ) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        vault.authority = authority;
        vault.senior_total_deposits = 0;
        vault.junior_total_deposits = 0;
        vault.senior_shares_supply = 0;
        vault.junior_shares_supply = 0;
        vault.senior_nav = 0;
        vault.junior_nav = 0;
        vault.senior_apy_cap_bps = senior_apy_cap_bps;
        vault.last_yield_ts = 0;
        vault.bump = ctx.bumps.vault; // ✅ Anchor 0.30+ style
        Ok(())
    }

    pub fn deposit_senior(ctx: Context<Deposit>, amount_usd_fp: u128) -> Result<()> {
        require!(amount_usd_fp > 0, TrancheError::InvalidAmount);

        let vault = &mut ctx.accounts.vault;
        let pos = &mut ctx.accounts.position;
        let user = &ctx.accounts.user;

        // Price-per-share (PPS)
        let pps = if vault.senior_shares_supply == 0 {
            FP_SCALE
        } else {
            checked_mul_div(vault.senior_nav, FP_SCALE, vault.senior_shares_supply)?
        };

        // Shares to mint
        let shares_out = checked_mul_div(amount_usd_fp, FP_SCALE, pps)?;
        require!(shares_out > 0, TrancheError::ZeroShares);

        // Update vault accounting
        vault.senior_total_deposits = vault
            .senior_total_deposits
            .checked_add(amount_usd_fp)
            .ok_or(TrancheError::MathOverflow)?;
        vault.senior_nav = vault
            .senior_nav
            .checked_add(amount_usd_fp)
            .ok_or(TrancheError::MathOverflow)?;
        vault.senior_shares_supply = vault
            .senior_shares_supply
            .checked_add(shares_out)
            .ok_or(TrancheError::MathOverflow)?;

        // Update user position
        ensure_owner(pos, user.key())?;
        pos.senior_shares = pos
            .senior_shares
            .checked_add(shares_out)
            .ok_or(TrancheError::MathOverflow)?;

        emit!(Deposited {
            user: user.key(),
            tranche: Tranche::Senior,
            amount_usd_fp,
            shares_minted: shares_out
        });

        Ok(())
    }

    pub fn deposit_junior(ctx: Context<Deposit>, amount_usd_fp: u128) -> Result<()> {
        require!(amount_usd_fp > 0, TrancheError::InvalidAmount);

        let vault = &mut ctx.accounts.vault;
        let pos = &mut ctx.accounts.position;
        let user = &ctx.accounts.user;

        let pps = if vault.junior_shares_supply == 0 {
            FP_SCALE
        } else {
            checked_mul_div(vault.junior_nav, FP_SCALE, vault.junior_shares_supply)?
        };

        let shares_out = checked_mul_div(amount_usd_fp, FP_SCALE, pps)?;
        require!(shares_out > 0, TrancheError::ZeroShares);

        vault.junior_total_deposits = vault
            .junior_total_deposits
            .checked_add(amount_usd_fp)
            .ok_or(TrancheError::MathOverflow)?;
        vault.junior_nav = vault
            .junior_nav
            .checked_add(amount_usd_fp)
            .ok_or(TrancheError::MathOverflow)?;
        vault.junior_shares_supply = vault
            .junior_shares_supply
            .checked_add(shares_out)
            .ok_or(TrancheError::MathOverflow)?;

        ensure_owner(pos, user.key())?;
        pos.junior_shares = pos
            .junior_shares
            .checked_add(shares_out)
            .ok_or(TrancheError::MathOverflow)?;

        emit!(Deposited {
            user: user.key(),
            tranche: Tranche::Junior,
            amount_usd_fp,
            shares_minted: shares_out
        });

        Ok(())
    }

    /// Distribute positive yield across tranches with a per-period cap to Senior.
    /// - Senior receives up to `min(yield_fp, senior_cap_fp)`
    /// - Remainder goes to Junior
    pub fn distribute_yield(ctx: Context<OnlyVaultMut>, yield_fp: u128) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        require!(yield_fp > 0, TrancheError::InvalidAmount);

        // Senior cap for "this period"
        let senior_cap_fp = checked_mul_div(
            vault.senior_nav,
            vault.senior_apy_cap_bps as u128,
            BPS_DENOM,
        )?;

        let senior_gain = core::cmp::min(yield_fp, senior_cap_fp);
        let junior_gain = yield_fp
            .checked_sub(senior_gain)
            .ok_or(TrancheError::MathOverflow)?;

        vault.senior_nav = vault
            .senior_nav
            .checked_add(senior_gain)
            .ok_or(TrancheError::MathOverflow)?;
        vault.junior_nav = vault
            .junior_nav
            .checked_add(junior_gain)
            .ok_or(TrancheError::MathOverflow)?;

        vault.last_yield_ts = Clock::get()?.unix_timestamp;

        emit!(YieldDistributed {
            senior_gain_fp: senior_gain,
            junior_gain_fp: junior_gain,
            senior_capped_gain_fp: senior_cap_fp,
            surplus_to_junior_fp: junior_gain
        });

        Ok(())
    }

    /// Authority-only: simulate a loss hitting Junior first, then Senior.
    pub fn simulate_loss(ctx: Context<AuthOnly>, total_loss_fp: u128) -> Result<()> {
        require!(total_loss_fp > 0, TrancheError::InvalidAmount);

        let vault = &mut ctx.accounts.vault;
        require_keys_eq!(ctx.accounts.authority.key(), vault.authority, TrancheError::Unauthorized);

        // Junior absorbs first
        let junior_absorb = core::cmp::min(total_loss_fp, vault.junior_nav);
        vault.junior_nav = vault
            .junior_nav
            .checked_sub(junior_absorb)
            .ok_or(TrancheError::MathOverflow)?;

        let remaining = total_loss_fp
            .checked_sub(junior_absorb)
            .ok_or(TrancheError::MathOverflow)?;

        // Senior absorbs remainder
        let senior_absorb = core::cmp::min(remaining, vault.senior_nav);
        vault.senior_nav = vault
            .senior_nav
            .checked_sub(senior_absorb)
            .ok_or(TrancheError::MathOverflow)?;

        emit!(LossApplied {
            total_loss_fp,
            absorbed_by_junior_fp: junior_absorb,
            absorbed_by_senior_fp: senior_absorb
        });

        emit!(SimulatedLoss {
            amount_usd_fp: total_loss_fp
        });

        Ok(())
    }

    /// Authority-only: simulate yield surplus by applying the same logic as `distribute_yield`.
    pub fn simulate_yield_surplus(ctx: Context<AuthOnly>, amount_usd_fp: u128) -> Result<()> {
        require!(amount_usd_fp > 0, TrancheError::InvalidAmount);

        let vault = &mut ctx.accounts.vault;
        require_keys_eq!(ctx.accounts.authority.key(), vault.authority, TrancheError::Unauthorized);

        emit!(SimulatedYield { amount_usd_fp });

        // Same math as distribute_yield, but using AuthOnly's mutable vault.
        let senior_cap_fp = checked_mul_div(
            vault.senior_nav,
            vault.senior_apy_cap_bps as u128,
            BPS_DENOM,
        )?;

        let senior_gain = core::cmp::min(amount_usd_fp, senior_cap_fp);
        let junior_gain = amount_usd_fp
            .checked_sub(senior_gain)
            .ok_or(TrancheError::MathOverflow)?;

        vault.senior_nav = vault
            .senior_nav
            .checked_add(senior_gain)
            .ok_or(TrancheError::MathOverflow)?;
        vault.junior_nav = vault
            .junior_nav
            .checked_add(junior_gain)
            .ok_or(TrancheError::MathOverflow)?;
        vault.last_yield_ts = Clock::get()?.unix_timestamp;

        emit!(YieldDistributed {
            senior_gain_fp: senior_gain,
            junior_gain_fp: junior_gain,
            senior_capped_gain_fp: senior_cap_fp,
            surplus_to_junior_fp: junior_gain
        });

        Ok(())
    }
}

/* ------------------------------- Accounts -------------------------------- */

#[derive(Accounts)]
#[instruction(authority: Pubkey)]
pub struct InitializeVault<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + TrancheVault::LEN,
        seeds = [b"tranche_vault", authority.as_ref()],
        bump
    )]
    pub vault: Account<'info, TrancheVault>,

    /// Payer for initialization
    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub vault: Account<'info, TrancheVault>,

    /// User position PDA (init if needed)
    #[account(
        init_if_needed,
        payer = user,
        space = 8 + UserPosition::LEN,
        seeds = [b"position", vault.key().as_ref(), user.key().as_ref()],
        bump
    )]
    pub position: Account<'info, UserPosition>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct OnlyVaultMut<'info> {
    #[account(mut)]
    pub vault: Account<'info, TrancheVault>,
}

#[derive(Accounts)]
pub struct AuthOnly<'info> {
    #[account(mut)]
    pub vault: Account<'info, TrancheVault>,
    pub authority: Signer<'info>,
}

/* -------------------------------- State ---------------------------------- */

#[account]
pub struct TrancheVault {
    pub authority: Pubkey,
    pub senior_total_deposits: u128,
    pub junior_total_deposits: u128,
    pub senior_shares_supply: u128,
    pub junior_shares_supply: u128,
    pub senior_nav: u128,
    pub junior_nav: u128,
    pub senior_apy_cap_bps: u16,
    pub last_yield_ts: i64,
    pub bump: u8,
    // padding for future upgrades (align to 8)
    pub _reserved0: [u8; 5],
}

impl TrancheVault {
    pub const LEN: usize =
        32 + // authority
        16 + // senior_total_deposits
        16 + // junior_total_deposits
        16 + // senior_shares_supply
        16 + // junior_shares_supply
        16 + // senior_nav
        16 + // junior_nav
        2  + // senior_apy_cap_bps
        8  + // last_yield_ts
        1  + // bump
        5;   // _reserved0
}

#[account]
pub struct UserPosition {
    pub owner: Pubkey,
    pub senior_shares: u128,
    pub junior_shares: u128,
}

impl UserPosition {
    pub const LEN: usize =
        32 + // owner
        16 + // senior_shares
        16;  // junior_shares
}

/* --------------------------------- Types --------------------------------- */

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum Tranche {
    Senior,
    Junior,
}

/* -------------------------------- Events --------------------------------- */

#[event]
pub struct Deposited {
    pub user: Pubkey,
    pub tranche: Tranche,
    pub amount_usd_fp: u128,
    pub shares_minted: u128,
}

#[event]
pub struct YieldDistributed {
    pub senior_gain_fp: u128,
    pub junior_gain_fp: u128,
    pub senior_capped_gain_fp: u128,
    pub surplus_to_junior_fp: u128,
}

#[event]
pub struct LossApplied {
    pub total_loss_fp: u128,
    pub absorbed_by_junior_fp: u128,
    pub absorbed_by_senior_fp: u128,
}

#[event]
pub struct SimulatedYield {
    pub amount_usd_fp: u128,
}

#[event]
pub struct SimulatedLoss {
    pub amount_usd_fp: u128,
}

/* -------------------------------- Errors --------------------------------- */

#[error_code]
pub enum TrancheError {
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("Zero shares minted")]
    ZeroShares,
    #[msg("Senior cap exceeded")]
    CapExceeded,
}

/* ------------------------------- Utilities -------------------------------- */

#[inline]
fn ensure_owner(pos: &mut Account<UserPosition>, owner: Pubkey) -> Result<()> {
    if pos.owner == Pubkey::default() {
        pos.owner = owner;
        return Ok(());
    }
    require_keys_eq!(pos.owner, owner, TrancheError::Unauthorized);
    Ok(())
}

/// Compute (a * b) / c with checked ops, rounding down.
#[inline]
fn checked_mul_div(a: u128, b: u128, c: u128) -> Result<u128> {
    require!(c != 0, TrancheError::InvalidAmount);
    let prod = a.checked_mul(b).ok_or(TrancheError::MathOverflow)?;
    Ok(prod.checked_div(c).ok_or(TrancheError::MathOverflow)?)
}

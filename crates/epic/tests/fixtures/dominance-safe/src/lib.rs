use anchor_lang::prelude::*;

declare_id!("11111111111111111111111111111111");

/// EPIC-SEC-002 no-finding fixture — unconditional signer check dominates the write.
///
/// The `require!` is unconditional and precedes the write on every execution path,
/// so the write is fully dominated. EPIC-SEC-002 must NOT fire here.
///
/// Expected: no findings.
#[program]
pub mod dominance_safe {
    use super::*;

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        // Unconditional guard — dominates all paths to the write below.
        require!(ctx.accounts.authority.is_signer, ErrorCode::Unauthorized);
        // Compound-assignment write — safely dominated.
        ctx.accounts.vault.balance -= amount;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub vault: Account<'info, VaultAccount>,
}

#[account]
pub struct VaultAccount {
    pub balance: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized")]
    Unauthorized,
}

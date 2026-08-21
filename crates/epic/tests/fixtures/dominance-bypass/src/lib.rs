use anchor_lang::prelude::*;

declare_id!("11111111111111111111111111111111");

/// EPIC-SEC-002 demo fixture — dominance bypass via conditional signer check.
///
/// The account `authority` is an unchecked `AccountInfo<'info>`.
/// The `require!` lives inside `if some_condition { ... }`, so the privileged
/// write `ctx.accounts.vault.balance -= amount` on line 23 is NOT dominated by
/// the signer check. An attacker can call with `some_condition = false` and
/// bypass the check entirely.
///
/// Expected: EPIC-SEC-002 fires for `authority`.
#[program]
pub mod dominance_bypass {
    use super::*;

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64, some_condition: bool) -> Result<()> {
        if some_condition {
            require!(ctx.accounts.authority.is_signer, ErrorCode::Unauthorized);
        }
        // Privileged compound-assignment write — NOT dominated by the conditional check above.
        ctx.accounts.vault.balance -= amount;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    /// CHECK: unsafe unchecked authority account checked manually in handler
    #[account(mut)]
    pub authority: AccountInfo<'info>,
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

use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod vulnerable_program {
    use super::*;

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        // Vulnerable: vault is a raw AccountInfo and lacks owner check!
        let mut vault_data = vault.try_borrow_mut_data()?;
        vault_data[0] = 9;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    /// CHECK: This is unsafe because it is UncheckedAccount and lacks any owner constraint/validation
    #[account(mut)]
    pub vault: AccountInfo<'info>,
    pub authority: Signer<'info>,
}

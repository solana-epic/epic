use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod edge_case_assault {
    use super::*;

    // 1. Alias resolution
    pub fn test_alias(ctx: Context<TestAccounts>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        let alias_vault = vault;
        let mut data = alias_vault.try_borrow_mut_data()?;
        data[0] = 1;
        Ok(())
    }

    // 2. Shadowed variables
    pub fn test_shadowing(ctx: Context<TestAccounts>) -> Result<()> {
        let mut target = &mut ctx.accounts.vault;
        {
            let target = &ctx.accounts.authority;
            // Inner target shadows the outer vault
        }
        let mut data = target.try_borrow_mut_data()?;
        data[0] = 1;
        Ok(())
    }

    // 3. Nested scopes
    pub fn test_nested_scopes(ctx: Context<TestAccounts>) -> Result<()> {
        let mut target = &mut ctx.accounts.vault;
        {
            let inner = &mut ctx.accounts.vault;
            let mut data = inner.try_borrow_mut_data()?;
            data[0] = 1;
        }
        Ok(())
    }

    // 4. Reassignments
    pub fn test_reassignment(ctx: Context<TestAccounts>) -> Result<()> {
        let mut target = &mut ctx.accounts.vault;
        target = &mut ctx.accounts.other_vault;
        let mut data = target.try_borrow_mut_data()?;
        data[0] = 1;
        Ok(())
    }

    // 5. Remaining accounts
    pub fn test_remaining_accounts(ctx: Context<TestAccounts>) -> Result<()> {
        let rem_acc = &ctx.remaining_accounts[0];
        let mut data = rem_acc.try_borrow_mut_data()?;
        data[0] = 1;
        Ok(())
    }

    // 6. Dynamic owner check
    pub fn test_dynamic_owner_check(ctx: Context<TestAccounts>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        if vault.owner != &crate::ID {
            return Err(ProgramError::InvalidAccountOwner.into());
        }
        let mut data = vault.try_borrow_mut_data()?;
        data[0] = 1;
        Ok(())
    }

    // 7. PDA derived validation
    pub fn test_pda_derivation(ctx: Context<TestAccounts>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        let expected_pda = Pubkey::create_program_address(&[b"vault"], &crate::ID)?;
        if vault.key() != expected_pda {
            return Err(ProgramError::InvalidArgument.into());
        }
        let mut data = vault.try_borrow_mut_data()?;
        data[0] = 1;
        Ok(())
    }

    // 8. Explicit return paths
    pub fn test_explicit_return(ctx: Context<TestAccounts>, mode: u8) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        if mode == 0 {
            return Ok(());
        }
        let mut data = vault.try_borrow_mut_data()?;
        data[0] = 1;
        Ok(())
    }

    // 9. Panic paths
    pub fn test_panic_path(ctx: Context<TestAccounts>, val: u8) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        if val == 0 {
            panic!("invalid value");
        }
        let mut data = vault.try_borrow_mut_data()?;
        data[0] = 1;
        Ok(())
    }

    // 10. Try operator paths
    pub fn test_try_operator(ctx: Context<TestAccounts>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        let mut data = vault.try_borrow_mut_data()?;
        data[0] = 1;
        Ok(())
    }

    // 11. Match expressions
    pub fn test_match_expression(ctx: Context<TestAccounts>, val: Option<u8>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        match val {
            Some(v) => {
                let mut data = vault.try_borrow_mut_data()?;
                data[0] = v;
            }
            None => {}
        }
        Ok(())
    }

    // 12. Loop structures
    pub fn test_loop_structure(ctx: Context<TestAccounts>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        for i in 0..10 {
            let mut data = vault.try_borrow_mut_data()?;
            data[i] = 1;
        }
        Ok(())
    }
}

#[derive(Accounts)]
pub struct TestAccounts<'info> {
    #[account(mut)]
    pub vault: AccountInfo<'info>,
    #[account(mut)]
    pub other_vault: AccountInfo<'info>,
    pub authority: Signer<'info>,
}

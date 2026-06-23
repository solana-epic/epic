# EPIC-SEC-002: Missing Signer Validation — Rule Specification

## Metadata
*   **Rule ID**: `EPIC-SEC-002`
*   **Title**: Missing Signer Validation
*   **Severity**: `CRITICAL`
*   **Threat Model**: An attacker supplies an arbitrary account in place of an authority-like account because no signer validation exists. This allows the attacker to execute privileged administrative actions, mutate sensitive state, or withdraw program funds.

---

## 1. Vulnerable Examples

### Native Solana Pattern (Unchecked Signer)
In native Solana, the account info is unpacked, but the code modifies the vault's state or config without verifying that the authority account is a signer of the transaction.
```rust
pub fn update_config(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let config_info = next_account_info(account_iter)?;
    let authority_info = next_account_info(account_iter)?; // Lacks .is_signer check!

    let mut config_data = config_info.try_borrow_mut_data()?;
    config_data[0] = 1; // Privileged write
    Ok(())
}
```

### Anchor Pattern (Missing Signer Type/Constraint)
Here, the accounts struct declares the `authority` as a standard `AccountInfo` or `UncheckedAccount` without the `signer` constraint or wrapping it in the `Signer` type.
```rust
#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(mut)]
    pub config: Account<'info, ProgramConfig>,
    pub authority: AccountInfo<'info>, // Lacks #[account(signer)] or Signer<'info>!
}

pub fn update_config(ctx: Context<UpdateConfig>, new_val: u64) -> Result<()> {
    // Mutates state, but authority was not verified as a signer!
    ctx.accounts.config.admin_value = new_val;
    Ok(())
}
```

---

## 2. Safe Examples

### Native Solana Pattern (Explicit Signer Guard)
```rust
pub fn update_config(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let config_info = next_account_info(account_iter)?;
    let authority_info = next_account_info(account_iter)?;

    if !authority_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature.into());
    }

    let mut config_data = config_info.try_borrow_mut_data()?;
    config_data[0] = 1; 
    Ok(())
}
```

### Anchor Signer Type
```rust
#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(mut)]
    pub config: Account<'info, ProgramConfig>,
    pub authority: Signer<'info>, // Safe: verified by Anchor code generator
}
```

### Anchor Signer Attribute Constraint
```rust
#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(mut)]
    pub config: Account<'info, ProgramConfig>,
    #[account(signer)]
    pub authority: AccountInfo<'info>, // Safe: verified by Anchor constraint
}
```

---

## 3. Edge Cases

### Variable Aliasing
If an authority account is loaded into a local variable or a reference alias, validation on the alias must be recognized as valid for the source.
```rust
pub fn update_config(ctx: Context<UpdateConfig>) -> Result<()> {
    let auth = &ctx.accounts.authority;
    if !auth.is_signer {
        return Err(ErrorCode::MissingSignature.into());
    }
    ctx.accounts.config.admin_value = 100;
    Ok(())
}
```

### Dominance Check (Validation Dominates State Mutation)
**Unsafe (Write Dominates Validation)**:
```rust
pub fn update_config(ctx: Context<UpdateConfig>) -> Result<()> {
    ctx.accounts.config.admin_value = 100; // Unsafe: Mutates before check
    if !ctx.accounts.authority.is_signer {
        return Err(ErrorCode::MissingSignature.into());
    }
    Ok(())
}
```
**Safe (Validation Dominates Write)**:
```rust
pub fn update_config(ctx: Context<UpdateConfig>) -> Result<()> {
    if !ctx.accounts.authority.is_signer {
        return Err(ErrorCode::MissingSignature.into());
    }
    ctx.accounts.config.admin_value = 100; // Safe: Dominated by signer check
    Ok(())
}
```

### Panic / Abort Paths
If validation is performed and non-signing paths terminate immediately (via `panic!`, `assert!`, `require!`, or `?` operators), the rest of the flow is safe.
```rust
pub fn update_config(ctx: Context<UpdateConfig>) -> Result<()> {
    require!(ctx.accounts.authority.is_signer, ErrorCode::MissingSignature);
    ctx.accounts.config.admin_value = 100; // Safe: panic/abort dominates
    Ok(())
}
```

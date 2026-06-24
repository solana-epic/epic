# EPIC-SEC-003 Rule Specification

*   **Rule ID**: EPIC-SEC-003
*   **Title**: Missing Post-CPI Account Reload
*   **Severity**: CRITICAL
*   **Vulnerability Class**: Stale Cache State / Missing Sync.

---

## Threat Model
A Solana program invokes an external contract via CPI (e.g. transferring token balances, minting tokens, or updating vault parameters). The Solana runtime updates the on-chain account data. However, the calling program maintains a cached copy of the account's deserialized data in memory. If the calling program subsequently reads from or writes to that account without reloading its state from the blockchain, it operates on stale layout state.
*   An attacker can exploit this to perform double-spend operations or overwrite the balance updates performed during the CPI.

---

## Code Examples

### 1. Vulnerable Example (Anchor)
The following code performs a token transfer via CPI, then directly modifies the local account field without reloading the account from the chain.
```rust
pub fn process_withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    // CPI Call (mutates on-chain state)
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_accounts = Transfer {
        from: ctx.accounts.vault.to_account_info(),
        to: ctx.accounts.user_token.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::transfer(cpi_ctx, amount)?;

    // State access after CPI without reload (VULNERABLE!)
    ctx.accounts.vault.amount -= amount; 
    Ok(())
}
```

### 2. Safe Example (Anchor)
Reload is invoked immediately after the CPI call to sync the in-memory cache with the on-chain updates.
```rust
pub fn process_withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_accounts = Transfer {
        from: ctx.accounts.vault.to_account_info(),
        to: ctx.accounts.user_token.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::transfer(cpi_ctx, amount)?;

    // Safe: Account is reloaded
    ctx.accounts.vault.reload()?;

    ctx.accounts.vault.amount -= amount; 
    Ok(())
}
```

### 3. Edge Case: Conditional Branching (Vulnerable)
If a reload is only performed in one branch of an `if` expression, there exists a path that accesses the state without a reload.
```rust
pub fn process_withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    token::transfer(cpi_ctx, amount)?;

    if condition {
        ctx.accounts.vault.reload()?;
    }

    // Vulnerable: if condition was false, we access stale vault state!
    if ctx.accounts.vault.amount > 100 {
        msg!("Vault is rich");
    }
    Ok(())
}
```

### 4. Edge Case: Helper Functions and Aliasing (Safe)
EPIC traces verification facts across aliases and nested references.
```rust
pub fn process_withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    token::transfer(cpi_ctx, amount)?;

    let v = &mut ctx.accounts.vault;
    v.reload()?; // Safe: Reloading through alias

    v.amount -= amount;
    Ok(())
}
```

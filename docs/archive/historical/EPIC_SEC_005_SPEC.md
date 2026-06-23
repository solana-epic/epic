# EPIC-SEC-005 Rule Specification: Arbitrary CPI Target Program Spoofing

* **Rule ID**: EPIC-SEC-005
* **Title**: Arbitrary CPI Target Program Spoofing
* **Severity**: CRITICAL
* **Vulnerability Class**: Insecure CPI Target Verification

---

## Threat Model
In Solana, instructions are completely stateless and accept a list of accounts supplied by the caller, including the executable program accounts that the program interacts with. When a program performs a Cross-Program Invocation (CPI) to an external program (e.g., calling SPL Token transfer), it passes the target program's account info. 

If the program fails to verify that the target program account key matches the trusted, expected program ID (such as the official SPL Token program address), an attacker can pass a custom, malicious program. When the program invokes CPI on the attacker-supplied program, the malicious program executes arbitrary code under the authority of the calling program (or its PDAs), allowing the attacker to steal funds, falsify state, or hijack execution.

---

## Code Examples

### 1. Vulnerable Example (Native Solana)
Here, the program calls `invoke` with `token_program` directly without verifying that `token_program.key` equals the SPL Token program ID.
```rust
pub fn process_cpi_transfer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let source_info = next_account_info(account_info_iter)?;
    let dest_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?; // Untrusted target!

    let ix = spl_token::instruction::transfer(
        token_program.key,
        source_info.key,
        dest_info.key,
        authority_info.key,
        &[],
        amount,
    )?;

    // VULNERABLE: Invoking CPI on an attacker-supplied program
    invoke(
        &ix,
        &[source_info.clone(), dest_info.clone(), authority_info.clone(), token_program.clone()]
    )?;
    Ok(())
}
```

### 2. Safe Example (Native Solana with Runtime Guard)
An imperative check is added to ensure that the `token_program.key` matches the expected standard SPL Token Program ID before execution.
```rust
pub fn process_cpi_transfer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let source_info = next_account_info(account_info_iter)?;
    let dest_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let token_program = next_account_info(account_info_iter)?;

    // SAFE: The check strictly dominates the invoke statement
    if token_program.key != &spl_token::ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    let ix = spl_token::instruction::transfer(
        token_program.key,
        source_info.key,
        dest_info.key,
        authority_info.key,
        &[],
        amount,
    )?;

    invoke(
        &ix,
        &[source_info.clone(), dest_info.clone(), authority_info.clone(), token_program.clone()]
    )?;
    Ok(())
}
```

### 3. Vulnerable Example (Anchor)
The program accepts `token_program` as an unvalidated `AccountInfo` and invokes a CPI through it.
```rust
pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let cpi_program = ctx.accounts.token_program.to_account_info(); // Untrusted AccountInfo
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_vault.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::transfer(cpi_ctx, amount)?;
    Ok(())
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub vault: Account<'info, VaultState>,
    pub user_vault: AccountInfo<'info>,
    pub authority: Signer<'info>,
    pub token_program: AccountInfo<'info>, // Untrusted program account!
}
```

### 4. Safe Example (Anchor with Program Wrapper)
By using `Program<'info, Token>` instead of `AccountInfo<'info>`, Anchor statically generates a check validating that `token_program` matches the SPL Token program ID.
```rust
pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_vault.to_account_info(),
        to: ctx.accounts.vault.to_account_info(),
        authority: ctx.accounts.authority.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::transfer(cpi_ctx, amount)?;
    Ok(())
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub vault: Account<'info, VaultState>,
    pub user_vault: AccountInfo<'info>,
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>, // SAFE: Anchor validates program address
}
```

---

## Edge Cases

### A. Alias Redefinitions (Safe)
```rust
let p = ctx.accounts.token_program;
let alias = p;
require_keys_eq!(alias.key(), token::ID);
// SAFE: Checks on the alias are mapped to the root variable
token::transfer(CpiContext::new(alias.to_account_info(), accounts), amount)?;
```

### B. Out-of-Order Execution (Vulnerable)
If the verification occurs *after* the invocation, it fails to protect the CPI.
```rust
// VULNERABLE: Validation does not dominate the CPI call
token::transfer(CpiContext::new(ctx.accounts.token_program.to_account_info(), accounts), amount)?;
require_keys_eq!(ctx.accounts.token_program.key(), token::ID);
```

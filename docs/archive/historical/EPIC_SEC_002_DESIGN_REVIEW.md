# EPIC-SEC-002: Missing Signer Validation — Hostile Design Review

## 1. What exactly is an authority-like account?
An **authority-like account** is any account passed to a Solana instruction that represents a privileged entity with the power to:
- Mutate program or account state (e.g., update config, write to registries).
- Authorize privileged actions (e.g., mint tokens, freeze assets, trigger emergency pauses).
- Transfer, withdraw, or burn funds.
- Upgrade the program, manage Program Derived Addresses (PDAs), or execute administrative flows.

In Solana's programming model, callers supply all account inputs. Because any caller can pass arbitrary public keys, the program *must* verify that the authority-like account signed the transaction. Failing to perform this signer validation allows an attacker to spoof the authority account with a key they control, bypassing authorization checks.

---

## 2. Which account names should never be trusted automatically?
The following account names are common heuristic markers for authority verification and should **never** be trusted without explicit signer validation:
- `authority`
- `admin`
- `owner`
- `manager`
- `signer`
- `initializer`
- `payer` (often reused as authority)
- `multisig`
- `delegate`
- `operator`
- `whitelist` / `blacklist` manager
- `treasury_owner`
- `user` (when the instruction performs actions on behalf of users)

---

## 3. Which Anchor constraints count as signer validation?
Under the Anchor framework, two primary constructs enforce structural signer validation:
1. **The `Signer<'info>` Type Wrapper**:
   Declaring an account as `Signer<'info>` inside an accounts struct:
   ```rust
   pub authority: Signer<'info>
   ```
2. **The `#[account(signer)]` Attribute**:
   Adding the `signer` constraint to any account type (such as `AccountInfo<'info>` or `Account<'info, T>`):
   ```rust
   #[account(signer)]
   pub authority: AccountInfo<'info>
   ```

Both constraints instruct Anchor's code generator to insert checks verifying that the account's `is_signer` flag is true before executing the instruction handler.

---

## 4. Which runtime checks count as signer validation?
In native Solana programs, or when bypassing Anchor's structural declarations, programs must validate signer status at runtime:
- **Direct Boolean Flag Checks**:
  Evaluating `authority.is_signer` or checking it on an alias.
- **Assertion / Guard Macros**:
  Using `require!(authority.is_signer, ...)` or `assert!(authority.is_signer)`.
- **Conditional Aborts**:
  Explicitly returning an error if the signer check fails:
  ```rust
  if !authority.is_signer {
      return Err(ProgramError::MissingRequiredSignature.into());
  }
  ```
- **Helper Functions / Key Relations**:
  Validating that a verified signer (e.g. `payer.is_signer == true`) matches or is cryptographically authorized to act on behalf of the target authority.

---

## 5. Which situations must return INCONCLUSIVE?
To maintain high precision, EPIC should mark validation as `INCONCLUSIVE` (or generate warnings rather than critical blocks) in these ambiguous scenarios:
- **Unresolved Helper Functions**: Signer validation occurs inside an external helper function or library call whose implementation source code is not available in the current AST.
- **Loop-Bound Checks**: The validation logic is located inside a loop or dynamic `match` block whose termination and execution guarantees cannot be fully statically proven.
- **Conditional Administrative State**: Validation is only required under specific transaction configurations (e.g., `if amount > LIMIT { require!(authority.is_signer); }`).
- **Signature Delegation via PDA/CPI**: Operations authorized via program-derived addresses where signature validation is delegated to a calling program.

---

## 6. Which situations produce false positives in Sentio and Anchor-Sentinel?
Simple pattern-matching security tools (like Sentio and Anchor-Sentinel) generate high rates of false positives in these common code patterns:
- **Validation inside Custom Helpers**: When a developer encapsulates signer checks in a shared utility (e.g., `fn check_authority(acc: &AccountInfo) -> Result<()>`), pattern matchers scanning only the instruction handler fail to find the checks.
- **Variable Aliasing**: Reassigning accounts to local variables (e.g., `let auth = &ctx.accounts.admin;`) and validating the alias. Pattern matchers miss the connection.
- **Assertive Boolean Expressions**: Combining signer checks with other assertions in a single expression (e.g., `require!(x == y && authority.is_signer, Error)`), which basic parsing fails to deconstruct.
- **Try Operator and Early Returns**: Using helper macros or early returns via `?` that terminate execution early upon check failures.

---

## 7. How will EPIC leverage CFG, SSA, Dominance, and GuardFacts to outperform pattern matching?
EPIC uses compiler-grade static analysis to overcome the limitations of simple pattern matchers:
1. **Control Flow Graph (CFG)**: Represents all execution branches, including panic paths, early returns, and try-operator branches. EPIC verifies that every non-aborting execution path that accesses a privileged instruction passes through a validation point.
2. **Static Single Assignment (SSA) & Aliasing**: Tracks all assignments via a Write-Dependency Graph (WDG). If `authority` is aliased as `auth_alias`, the SSA engine resolves the alias chain back to its root account symbol, ensuring validations on `auth_alias` protect actions on `authority`.
3. **Dominance Engine**: Calculates dominance frontiers for the CFG. Instead of checking if a signer check exists *somewhere*, the Dominance Engine checks that the signer check block *dominates* the privileged mutation block. Any path to the write/action must flow through the signer check first.
4. **GuardFacts**: Normalizes both structural Anchor constraints and imperative runtime guards into unified semantic facts. The rule engine queries `GuardFacts::Signer` uniformly, separating rule logic from source syntax variations.

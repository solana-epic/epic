# EPIC-SEC-005 Hostile Design Review — Arbitrary CPI Target Program Validation

This document reviews the security implications, compile-time detection model, and false positive/negative vectors for the **EPIC-SEC-005 (Arbitrary CPI Target Program Spoofing)** security rule.

---

## 1. What exactly qualifies as a CPI?
A Cross-Program Invocation (CPI) occurs when a Solana program invokes another program on-chain. Syntactically, this includes:
1. **Solana Runtime Invocations**:
   * Direct calls to `solana_program::program::invoke` or `invoke_signed`.
   * Calls to standard utility wrappers like `invoke_unchecked`, `invoke_signed_unchecked`.
2. **Anchor CPI Helpers**:
   * Calling functions within Anchor program CPI modules (e.g., `token::transfer`, `token::mint_to`, `token::burn`, `associated_token::create`).
   * Explicit constructors of `CpiContext::new` or `CpiContext::new_with_signer` that package the target program account.
3. **Indirect/Dynamic CPI Invocations**:
   * Helper functions inside the instruction body that wrap `invoke` or `CpiContext::new` (transitive CPI calls).

---

## 2. What constitutes a trusted program?
A program is considered trusted if its address (public key) is validated to match a known, expected program. Standard programs that are implicitly trusted or require standard validation include:
* **Solana System Program** (`11111111111111111111111111111111`)
* **SPL Token Program** (`TokenkegQfeZyiMwAJb3nd6JQJkgHqv670XV1De8D`)
* **SPL Token-2022 Program** (`TokenzQdBNbkh1kv2g5xK21ZaJiw1vTkZ1kw1mZ`)
* **SPL Associated Token Program** (`ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8`)
* Any custom user-configured whitelisted program ID (e.g. checking against a program ID stored in the program's configuration/state account).

---

## 3. What forms of validation are acceptable?
The target program account must be validated through one of the following methods before being invoked:
1. **Static Type Constraints**:
   * Declaring the account as `Program<'info, T>` in Anchor, where `T` is a known program type (e.g., `Token`, `System`, `AssociatedToken`).
2. **Address Constraint Checks**:
   * Anchor account macro constraints: `#[account(address = token::ID)]` or similar ID constants.
   * Imperative equality checks: `program.key() == token::ID` or `*program.key == expected_id`.
3. **Imperative Custom Guards**:
   * Runtime assertions: `require_keys_eq!(program.key(), token::ID)` or `if program.key() != expected_id { return Err(...) }` that dominate the execution path.

---

## 4. Which validations are Anchor-enforced?
Anchor provides built-in mechanisms to validate program targets:
* **`Program<'info, T>` wrappers**: Anchor automatically verifies that the account's address matches the hardcoded program ID associated with the type `T`.
* **Struct Attribute Constraints**:
  * `#[account(address = <expected_id>)]`: Statically verifies the public key matches the target.
  * `#[account(constraint = <expression>)]`: Executes custom logic, such as checking `program.key() == expected_id`.

---

## 5. Which validations are runtime-enforced?
Runtime-enforced checks are imperative Rust assertions within the instruction function body:
* Checks using comparison operators: `program.key() == token::ID` or `program.to_account_info().key == &expected_id`.
* Control flow splits: `if program.key() != expected_id { return Err(ErrorCode::InvalidProgram.into()); }`.
* Runtime assertions: `assert_eq!(program.key(), token::ID)` or `require!(program.key() == expected_id, Error)`.

---

## 6. What are common false positives?
Standard pattern matchers often trigger false positives in the following scenarios:
1. **Implicitly Trusted System Programs**: System, Token, and Associated Token programs are often passed as raw `AccountInfo` but validated transitively or assumed to be safe.
2. **Alias References**: If the program is checked under one variable name (`let p = &ctx.accounts.token_program; require!(p.key() == token::ID);`), but invoked using another (`invoke(..., token_program)`), naive matchers flag it.
3. **Indirect Registry/Whitelist Checks**: The program key is matched against a whitelist vector (e.g., `whitelist.contains(&program.key())`), which does not match a simple equality pattern.
4. **Helper Functions**: The validation is performed in a preceding block or outer helper function, but the invocation occurs inside another function context.

---

## 7. What are common false negatives?
Vulnerabilities can slip past basic checks due to:
1. **Order of Execution Violations**: Checking the program address *after* performing the CPI or writing to state.
2. **Checking Owner Instead of Identity**: Verifying that the target program's owner is the BPF Upgradeable Loader. This only proves that the target is *a* program, not *the correct* program; an attacker can deploy their own malicious program.
3. **Mismatched Target**: Validating one program variable but passing a different, unvalidated program variable into the CPI instruction.
4. **Unconditional Path Merging**: A check is performed inside an `if` block, but the execution path merges and invokes the CPI regardless of whether the branch was taken.

---

## 8. How does Sentio detect SW003?
Sentio's SW003 rule utilizes pattern matching on AST inputs:
* It looks for function calls to `invoke` or `invoke_signed`.
* It performs basic text/AST pattern checks on the target program argument to see if it has been validated in the immediate context.
* It does not parse the Control Flow Graph (CFG) or calculate dominance, leading to false negatives on out-of-order execution and false positives on helper functions, alias chains, or complex branching paths.

---

## 9. How can EPIC use CFG/Dominance to outperform it?
EPIC uses its compiler-grade static analysis engine to eliminate these issues:
1. **Path-Sensitive Reachability**: EPIC performs a path-sensitive traversal of the CFG. Any path from the entry of the instruction to the CPI node that does not contain a validating guard fact generates a warning.
2. **Dominance Verification**: Ensures that the program validation check strictly *dominates* the CPI statement. If a state change or CPI occurs before the check, it is flagged as unsafe.
3. **SSA & Alias Tracing**: Resolves transitive assignments (e.g., `let alias = program;`). Validations on the alias or the root symbol are correctly tracked and aggregated.
4. **Symbol & Type Resolution**: Automatically detects and trust Anchor's `Program<'info, T>` types and extracts declared struct constraints directly into the rule's active GuardFacts registry.

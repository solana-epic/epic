# EPIC Sentio Rule Decomposition

This document decomposes and analyzes all 16 security rules implemented in the Sentio (`sentio-rs`) Solana auditing engine. It details the targeted vulnerability classes, Sentio's detection methodology, EPIC's existing coverage, and how EPIC's control-flow graph (CFG), static single assignment (SSA), dominance engine, and SymbolResolver can be leveraged to exceed Sentio's pattern-matching capabilities.

---

## Rule Analysis Matrix

### SW001: Missing Signer Check
*   **Vulnerability Class**: Missing Signer Validation / Privilege Escalation. An instruction performs privileged operations (e.g., modifying state, transferring funds) on behalf of an authority account without verifying that the authority signed the transaction.
*   **Sentio Detection**: Searches the AST for account struct fields typed as `AccountInfo` or `UncheckedAccount` with names containing `authority`, `admin`, `signer`, or `initializer`. It flags them if they lack `#[account(signer)]` or `address` constraints and there is no raw token substring match of the field name in the instruction body’s `is_signer` checks.
*   **EPIC Coverage**: Fully covered by `EPIC-SEC-002`.
*   **CFG/SSA/Dominance Advantage**: High. Sentio's token matcher skips alerts if the field identifier is checked *anywhere* in the file. This creates false negatives if the check occurs after a state mutation (violating dominance) or in a different instruction handler. EPIC's CFG ensures that the signer check dominates the specific write path. EPIC's SSA traces aliases (e.g., `let auth = ctx.accounts.authority; require!(auth.is_signer)`) which Sentio's string-matching misses.
*   **Expected False Positive Rate**: High in Sentio (due to naive name-matching and inability to resolve aliases/helpers); Low in EPIC.
*   **Engineering Complexity**: Already implemented.

---

### SW002: Missing Owner Check
*   **Vulnerability Class**: Missing Program Owner Validation. Passing a user-supplied account owned by a malicious program instead of the expected system/token/custom program, allowing state spoofing.
*   **Sentio Detection**: Scans for `AccountInfo` or `UncheckedAccount` fields in account structs that do not have `owner` or `address` constraints, and lack an owner guard in instruction logic. It ignores fields that appear to be data accounts (containing `init`, `init_if_needed`, `has_one`, `has_seeds`) or program accounts (names containing `program`).
*   **EPIC Coverage**: Fully covered by `EPIC-SEC-001`.
*   **CFG/SSA/Dominance Advantage**: High. Sentio cannot guarantee check order or verify that the check dominates the write. It also fails to trace owner validation through multi-file scopes or variable reassignments. EPIC uses path-sensitive dominance checks and traces writes back to their root accounts using the Write-Dependency Graph (WDG).
*   **Expected False Positive Rate**: High in Sentio; Low in EPIC.
*   **Engineering Complexity**: Already implemented.

---

### SW003: Arbitrary CPI Target
*   **Vulnerability Class**: Arbitrary CPI Target. Invoking a CPI call (`invoke` or `invoke_signed`) on an arbitrary, attacker-supplied program account rather than a hardcoded or validated program ID.
*   **Sentio Detection**: Inspects instructions for raw invocation calls (`invoke`, `invoke_signed`, or `invoke_unchecked`). It flags them if no guard referencing the program key precedes the CPI order index.
*   **EPIC Coverage**: Not covered.
*   **CFG/SSA/Dominance Advantage**: High. Sentio uses a simple linear order index comparison (`order < cpi_call.order`). It fails to handle complex branching (e.g., where a check is done in one branch but CPI occurs in another). EPIC’s CFG can prove that program ID validation dominates the CPI call across *all* execution paths.
*   **Expected False Positive Rate**: Medium in Sentio; Low in EPIC.
*   **Engineering Complexity**: Medium (Requires parsing raw invocation functions and mapping key-checks to `GuardFact`).

---

### SW005: Unchecked Arithmetic
*   **Vulnerability Class**: Integer Overflow/Underflow. Math operations (`+`, `-`, `*`, `+=`, `-=`, `*=`) that can wrap or panic, causing state corruption or denial of service.
*   **Sentio Detection**: Traverses the AST looking for binary arithmetic operators where at least one operand contains a dot `.` (suggesting a struct field access), while ignoring simple loop counters (e.g. `i += 1`).
*   **EPIC Coverage**: Not covered.
*   **CFG/SSA/Dominance Advantage**: Low/Medium. Arithmetic safety is not primarily a control-flow issue. However, EPIC’s type inference prevents false positives on non-numeric overloaded operators, and EPIC can parse `Cargo.toml` profiles to verify if `overflow-checks = true` is configured in release mode (which renders these alerts redundant since Rust checks all math at runtime in modern Solana builds).
*   **Expected False Positive Rate**: Extremely High in Sentio (flags safe math, projects with global overflow checks, and non-numeric types); Low in EPIC with Cargo profile check.
*   **Engineering Complexity**: Low (AST visitor) to Medium (with type integration).

---

### SW006: Type Cosplay — Missing Discriminator Check
*   **Vulnerability Class**: Type Cosplay. Deserializing account data without checking the type discriminator, allowing an attacker to pass an account of a different type but with a matching memory layout.
*   **Sentio Detection**: Scans for calls to `try_from_slice` and checks if the call arguments do not contain `"8.."` or `"discriminator"`.
*   **EPIC Coverage**: Anchor's `Account<'info, T>` prevents this by default (validated by EPIC's structural parser). Raw deserializations on `AccountInfo` are not currently checked.
*   **CFG/SSA/Dominance Advantage**: Medium. Sentio relies on a string check of the arguments. It false-positives if a slice is created on a separate line (e.g., `let slice = &data[8..]; try_from_slice(slice)`). EPIC's SSA engine traces the slice variable back to its parent array and validates the offset semantically.
*   **Expected False Positive Rate**: Medium in Sentio; Low in EPIC.
*   **Engineering Complexity**: Medium.

---

### SW008: Missing Post-CPI Account Reload
*   **Vulnerability Class**: Stale Account Cache. An instruction invokes a CPI that mutates an account, and subsequently reads or writes that account without invoking `.reload()?`, resulting in stale cached data overwriting the on-chain state.
*   **Sentio Detection**: Scans for functions containing both a CPI call and a subsequent write targeting the same account (or any field access write if names cannot be resolved), flagging if there is no intervening call to `.reload()`.
*   **EPIC Coverage**: Not covered.
*   **CFG/SSA/Dominance Advantage**: High. This is a path-sensitive ordering problem. Sentio's linear ordering fails to track branching (e.g., if a reload occurs inside an `if` block that guards the write). EPIC’s CFG can check all execution paths from the CPI node to the write node, ensuring that a reload instruction dominates the write.
*   **Expected False Positive Rate**: Medium in Sentio; Low in EPIC.
*   **Engineering Complexity**: Medium.

---

### SW009: Missing Token Account Mint Check
*   **Vulnerability Class**: Token Mint Spoofing. Accepting a token account with an attacker-controlled mint, permitting them to drain funds or spoof deposits.
*   **Sentio Detection**: Identifies mutable fields of type `Account` or `InterfaceAccount` wrapping a `TokenAccount` and flags them if they lack `token::mint`, `address`, or `associated_token` constraints, and are not initialized.
*   **EPIC Coverage**: Not covered.
*   **CFG/SSA/Dominance Advantage**: Medium. EPIC can verify structural constraints via its AST representation and use its `SymbolResolver` to verify if runtime checks like `require!(token_acc.mint == expected_mint)` are present in the handler.
*   **Expected False Positive Rate**: Low in Sentio; Very Low in EPIC.
*   **Engineering Complexity**: Low.

---

### SW010: Missing Token Account Owner Check
*   **Vulnerability Class**: Token Owner Spoofing. Failing to verify that a token account belongs to the expected authority, allowing an attacker to substitute their own account.
*   **Sentio Detection**: Similar to `SW009`, scans mutable `TokenAccount` fields and flags them if they lack `token::authority`, `address`, `associated_token`, or `has_one = authority/owner` constraints.
*   **EPIC Coverage**: Not covered.
*   **CFG/SSA/Dominance Advantage**: Medium. EPIC can resolve custom authority validation logic within helper functions and ensure it dominates the token transfers.
*   **Expected False Positive Rate**: Low in Sentio; Very Low in EPIC.
*   **Engineering Complexity**: Low.

---

### SW011: AccountInfo Used as Data Account
*   **Vulnerability Class**: Missing Typed Deserialization. Using raw `AccountInfo` for a state account, which bypasses Anchor’s automatic owner and discriminator validations.
*   **Sentio Detection**: Flags any `AccountInfo` field in accounts structs that has data constraints like `init`, `init_if_needed`, `owner`, `address`, `has_one`, or `has_seeds`.
*   **EPIC Coverage**: Not covered.
*   **CFG/SSA/Dominance Advantage**: None. This is a purely static structural check.
*   **Expected False Positive Rate**: Extremely Low.
*   **Engineering Complexity**: Low.

---

### SW012: Missing Seeds + Bump on PDA
*   **Vulnerability Class**: Incorrect PDA Derivation. Specifying seeds without a bump, or a bump without seeds, which permits arbitrary accounts to bypass PDA derivation.
*   **Sentio Detection**: Flags fields in accounts structs that contain `seeds = [...]` but lack `bump`, or contain `bump` but lack `seeds`.
*   **EPIC Coverage**: Not covered.
*   **CFG/SSA/Dominance Advantage**: None. Static structural validation.
*   **Expected False Positive Rate**: Low.
*   **Engineering Complexity**: Low.

---

### SW013: PDA Seed References Unvalidated Account
*   **Vulnerability Class**: PDA Seed Hijacking. Derbying a PDA using a seed that references an unvalidated `AccountInfo` or `UncheckedAccount`. An attacker can supply arbitrary public keys as seed inputs to hijack PDA lookups.
*   **Sentio Detection**: Parses `seeds = [...]` arrays, extracts the identifiers, and checks if any identifier matches another field in the accounts struct that is an unvalidated `AccountInfo`/`UncheckedAccount` (lacking `owner`, `address`, `signer`, or typed constraints).
*   **EPIC Coverage**: Not covered.
*   **CFG/SSA/Dominance Advantage**: Medium. EPIC's workspace type registry can identify if the referenced account is validated inside custom helper functions or via manual checks in instruction handlers.
*   **Expected False Positive Rate**: Medium in Sentio; Low in EPIC.
*   **Engineering Complexity**: Medium.

---

### SW014: PDA Bump May Not Be Canonical
*   **Vulnerability Class**: Non-Canonical PDA Bump / Multi-Bump Attack. Accepting a user-supplied bump rather than the canonical bump derived by the runtime, opening up second-preimage collision risks.
*   **Sentio Detection**: Flags PDA fields using `bump = some_value` where the value is a bare identifier rather than an account field (i.e. contains no `.`), implying it is not a stored, derived bump.
*   **EPIC Coverage**: Not covered.
*   **CFG/SSA/Dominance Advantage**: Medium. EPIC can resolve parameter aliases and track SSA values to see if the bump variable is validated or derived canonically.
*   **Expected False Positive Rate**: Medium in Sentio; Low in EPIC.
*   **Engineering Complexity**: Low.

---

### SW016: InitIfNeeded Usage
*   **Vulnerability Class**: Initialization Reentrancy / State Reset. Using `init_if_needed` can permit an attacker to re-initialize an account and clear its data if authority and seed constraints are loose.
*   **Sentio Detection**: Flags any occurrence of the `init_if_needed` attribute.
*   **EPIC Coverage**: Not covered.
*   **CFG/SSA/Dominance Advantage**: None. Static pattern checking.
*   **Expected False Positive Rate**: High (it warns on *all* usages, even when completely safe/intended).
*   **Engineering Complexity**: Low.

---

### SW018: Missing Realloc Zeroing
*   **Vulnerability Class**: Stale Memory Read. Reallocating an account without setting `realloc::zero = true`. The newly allocated memory space is not cleared and may contain stale data left by other accounts.
*   **Sentio Detection**: Flags fields using `realloc = ...` but lacking `realloc::zero = true`.
*   **EPIC Coverage**: Not covered.
*   **CFG/SSA/Dominance Advantage**: None. Static structural check.
*   **Expected False Positive Rate**: Very Low.
*   **Engineering Complexity**: Low.

---

### SW020: AccountInfo Used as CPI Target Program
*   **Vulnerability Class**: CPI Target Program Spoofing. Typing a target program as `AccountInfo` instead of `Program<'info, T>`, which skips executable flags and program ID checks.
*   **Sentio Detection**: Flags fields of type `AccountInfo` whose name contains `program` and which do not have data constraints.
*   **EPIC Coverage**: Not covered.
*   **CFG/SSA/Dominance Advantage**: Medium. EPIC can verify if the target program account has its key verified against hardcoded expected IDs in the handler code.
*   **Expected False Positive Rate**: Medium (due to name heuristic mismatches).
*   **Engineering Complexity**: Low.

---

### SW021: PDA Seed Collision Risk
*   **Vulnerability Class**: PDA Seed Collision. Adjacent variable-length seeds (e.g., name and symbol bytes) hash to the same PDA because `find_program_address` concatenates them without separators.
*   **Sentio Detection**: Identifies `seeds = [...]` arrays and flags adjacent variable-length seeds (e.g., ending with `.as_bytes()`, `.as_slice()`, `.as_ref()`) without a fixed-length separator in between.
*   **EPIC Coverage**: Not covered.
*   **CFG/SSA/Dominance Advantage**: Medium. EPIC can leverage type resolution to determine if the variables are actually fixed-width arrays (e.g. `[u8; 32]`) or aliases to fixed-length data, preventing false positives.
*   **Expected False Positive Rate**: Medium in Sentio; Low in EPIC.
*   **Engineering Complexity**: Medium.

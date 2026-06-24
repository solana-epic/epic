# EPIC Top 10 Next Rules (ROI Analysis)

This report lists the Top 10 security rules recommended for EPIC's roadmap, evaluated against key product-led growth metrics: Engineering Effort, Security Value, User Demand, False Positive (FP) Risk, and Differentiation Value.

---

## ROI Rankings Table

| Rank | Rule ID | Rule Title | Engineering Effort | Security Value | User Demand | FP Risk | Differentiation Value |
| :--- | :--- | :--- | :---: | :---: | :---: | :---: | :---: |
| **1** | **SW008** | Missing post-CPI account reload | Medium | Critical | High | Low | High |
| **2** | **SW021** | PDA seed collision risk | Medium | High | High | Low | High |
| **3** | **SW003** | Arbitrary CPI target | Medium | High | High | Low | Medium |
| **4** | **SW009** | Missing token account mint check | Low | High | High | Low | Medium |
| **5** | **SW010** | Missing token account owner check | Low | High | High | Low | Medium |
| **6** | **SW013** | PDA seed unvalidated account | Medium | High | Medium | Low | Medium |
| **7** | **SW006** | Type cosplay — try_from_slice offset | Medium | High | Medium | Low | Medium |
| **8** | **SW018** | Missing realloc::zero = true | Low | Medium | Medium | Very Low | Low |
| **9** | **SW011** | AccountInfo used as data account | Low | Medium | Medium | Low | Low |
| **10**| **SW012** | Missing seeds/bump on PDA | Low | Medium | Medium | Low | Low |

---

## Detailed Rule Breakdowns

### 1. SW008: Missing Post-CPI Account Reload
*   **Vulnerability Class**: Stale Account Cache.
*   **Why it's Rank 1**: DeFi protocols (like Drift and Marginfi) regularly invoke external programs (CPI) to update values and then perform local state updates. Forgetting to reload the local account state causes silent overrides of mutated state.
*   **EPIC Advantage**: This is a classic control-flow reachability problem. EPIC's CFG and Dominance engine can trace whether *every* path from the CPI invocation to the write statement contains a reload instruction. Sentio's simple regex or AST node ordering is blind to branch splits and generates significant false positives.

### 2. SW021: PDA Seed Collision Risk
*   **Vulnerability Class**: PDA Cryptographic Seed Collision.
*   **Why it's Rank 2**: Concatenating adjacent variable-length seeds (e.g. `name.as_bytes()` and `symbol.as_bytes()`) allows an attacker to collide two logically distinct accounts onto the same PDA (e.g. `"abc"`/`"de"` vs `"ab"`/`"cde"`).
*   **EPIC Advantage**: Sentio's rule naive string matching fails when developers use fixed-width variables or custom array types. EPIC's type resolution engine computes exact byte layouts, enabling it to distinguish variable-length slices from fixed-width arrays.

### 3. SW003: Arbitrary CPI Target
*   **Vulnerability Class**: Target Program Spoofing.
*   **Why it's Rank 3**: Raw CPI invocations (`invoke` or `invoke_signed`) that execute CPI on user-supplied accounts without program ID verification allow attackers to spoof system programs.
*   **EPIC Advantage**: EPIC's SymbolResolver can trace the program ID validation facts through SSA variables to ensure the verification dominates the call path.

### 4. SW009: Missing Token Account Mint Check
*   **Vulnerability Class**: SPL Token Mint Spoofing.
*   **Why it's Rank 4**: Under Anchor, a mutable `TokenAccount` field must be locked to a specific mint. Attacker-supplied token accounts with fake mints are the root cause of historical multi-million dollar hacks (e.g. Cashio).
*   **EPIC Advantage**: Structural checking on Anchor attributes is quick to implement and delivers immense baseline value.

### 5. SW010: Missing Token Account Owner Check
*   **Vulnerability Class**: SPL Token Owner/Authority Spoofing.
*   **Why it's Rank 5**: Similar to SW009, this prevents attackers from passing a token account they control as the protocol's expected vault.

### 6. SW013: PDA Seed References Unvalidated Account
*   **Vulnerability Class**: PDA Seed Hijacking.
*   **Why it's Rank 6**: If a PDA uses an unvalidated account key as a seed, the attacker can grind public keys to spoof PDA addresses.
*   **EPIC Advantage**: Traces validation properties from context fields using SSA.

### 7. SW006: Type Cosplay — try_from_slice Offset
*   **Vulnerability Class**: Layout Cosplay.
*   **Why it's Rank 7**: Deserializing data via `try_from_slice` without slicing off the first 8 bytes of the Anchor discriminator allows layout collisions.
*   **EPIC Advantage**: EPIC uses SSA to trace slice offsets back to their origin.

### 8. SW018: Missing realloc::zero = true
*   **Vulnerability Class**: Stale Memory Read.
*   **Why it's Rank 8**: Reallocating space in Solana without zeroing it out leaves old memory bytes accessible.
*   **EPIC Advantage**: Trivial structural audit rule with high safety ROI.

### 9. SW011: AccountInfo Used as Data Account
*   **Vulnerability Class**: Unsafe Raw Deserialization.
*   **Why it's Rank 9**: Prevents raw `AccountInfo` usage on accounts with structured constraints.

### 10. SW012: Missing seeds/bump on PDA
*   **Vulnerability Class**: Incorrect PDA Derivation.
*   **Why it's Rank 10**: Catches incomplete account definitions that specify seeds but miss bumps.

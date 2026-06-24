# EPIC Rule Priority Matrix

This priority matrix classifies the 16 Sentio baseline rules into four strategic tiers (A, B, C, and D) based on their security impact, developer adoption value, demo value, and integration ROI with EPIC's compiler engine.

---

## Priority Classification

| Tier | Code / Rule | Description / Vulnerability Class | Strategic Rationale |
| :--- | :--- | :--- | :--- |
| **A: MUST IMPLEMENT** | **SW003** | Arbitrary CPI target program validation | **High Impact**: A critical vulnerability class (e.g. Wormhole proxy spoofing). High demo value for showing path-sensitive program ID check dominance. |
| | **SW008** | Missing post-CPI account reload | **High Impact**: Stale data cache exploits are a major source of loss in DeFi protocols. Perfectly showcases EPIC's CFG and dominance capabilities over Sentio. |
| | **SW009** | Missing token account mint constraint | **High Impact**: Spl-token mint spoofing is a common exploit vector. Crucial for financial contract validation. |
| | **SW010** | Missing token account owner/authority check | **High Impact**: Stealing funds by substituting attacker-controlled token accounts. Fundamental token security. |
| | **SW013** | PDA seed references unvalidated account | **High Impact**: Seed-grinding attacks allow attackers to hijack PDA lookups. High security research visibility. |
| | **SW021** | PDA seed collision risk (adjacent variable seeds) | **High Impact**: Crypotographic collision vulnerability. High differentiation value for showing EPIC's type size extraction. |
| **B: SHOULD IMPLEMENT**| **SW006** | Type cosplay — missing discriminator checks | **Medium Impact**: Anchor handles this automatically via `Account<'info, T>`, but manual deserializations still exist in hybrid programs. |
| | **SW011** | AccountInfo used as data account | **Medium Impact**: Code hygiene check that guides developers towards secure typed Anchor wrappers. |
| | **SW012** | Missing seeds/bump constraints on PDA | **Medium Impact**: Structural hygiene validation. Prevents incomplete PDA patterns. |
| | **SW014** | PDA bump may not be canonical | **Medium Impact**: Good security hygiene, though automated bump derivation in modern Anchor limits its occurrence. |
| | **SW018** | Missing `realloc::zero = true` constraint | **Medium Impact**: Prevents stale memory data leakage in reallocated spaces. A simple structural check. |
| | **SW020** | AccountInfo used as CPI target program | **Medium Impact**: Helps flag raw CPI setups, though `Program<'info, T>` is standard in modern Anchor. |
| **C: ALREADY COVERED** | **SW001** | Missing signer validation | **Fully Covered**: EPIC-SEC-002 uses path-sensitive dominance and WDG alias tracing, far outperforming Sentio's AST check. |
| | **SW002** | Missing owner check | **Fully Covered**: EPIC-SEC-001 enforces path-sensitive owner checks using AST, CFG, and dominance. |
| **D: DO NOT IMPLEMENT**| **SW005** | Unchecked arithmetic operations | **Low ROI**: Naive AST matchers produce massive false positives. Modern Solana relies on Cargo-level overflow checks, making panics standard. |
| | **SW016** | Usage of `init_if_needed` attribute | **Low ROI**: Warns universally on all usages. Highly noisy, as many valid workflows require it and guard it properly. |

---

## Strategic Analysis of MUST IMPLEMENT (Tier A)

1.  **SW008 (Missing Post-CPI Account Reload)**:
    This is the ultimate showcase for EPIC's compiler engineering. Stale cache exploits are mathematically complex. To prove an account reload is missing, we must trace control flow from a CPI instruction to a subsequent state write, ensuring no reload dominates the write path. Naive AST rules fall flat on branch splits.
2.  **SW021 (PDA Seed Collision Risk)**:
    Sentio's rule false-positives when developer-passed seeds are fixed-width byte array variables or custom sized aliases. EPIC's type resolution engine computes exact byte sizes, allowing it to verify if adjacent seeds are truly variable-length or fixed-width, maximizing precision.
3.  **SW003 (Arbitrary CPI Target)**:
    CPI programs must be validated. An attacker replacing `token_program` with a dummy malicious contract can bypass payment checks. Validating that program address checks dominate `invoke` calls is key.
4.  **SW009 & SW010 (Token Mint & Owner Checks)**:
    These two rules constitute the core of token transfer safety. Protocols are frequently drained by passing fake token accounts.
5.  **SW013 (Unvalidated Seed Accounts)**:
    PDA seeds are trusted parameters. If a seed uses an unvalidated `AccountInfo` (e.g. `user_account.key()`), an attacker can pass any key, generating a valid PDA for a different context.

# EPIC Next 3 Rules Verdict

To transition EPIC from an interesting prototype to a "must-install" developer utility, we recommend implementing **exactly three** strategic rules next. These rules are chosen to maximize security impact, provide a killer demo, and leverage EPIC's unique compiler capabilities (CFG, SSA, Dominance, and Type Registry) to outperform naive pattern matchers.

---

## The Next 3 Rules Selection

1.  **EPIC-SEC-003: Stale Account Cache (Missing Post-CPI Reload)** (Derived from SW008)
2.  **EPIC-SEC-004: PDA Cryptographic Seed Collision** (Derived from SW021)
3.  **EPIC-SEC-005: Arbitrary CPI target Program Spoofing** (Derived from SW003)

---

## Detailed Evaluation & Justification

### Rule 1: EPIC-SEC-003 — Missing Post-CPI Account Reload
*   **Why it matters**: Solana accounts mutated in a CPI call must be reloaded before they are written to again. If a developer fails to reload, the local transaction buffer contains stale cached data. A subsequent local write will overwrite the CPI changes with outdated state, leading to critical exploits.
*   **Why users expect it**: Stale layout caches are extremely difficult to detect manually, and this vulnerability has affected major protocols (like Drift and Marginfi).
*   **How it complements EPIC-SEC-001 & EPIC-SEC-002**:
    *   `EPIC-SEC-001` validates **Account Owners** (Safe Inputs).
    *   `EPIC-SEC-002` validates **Transaction Signers** (Safe Authority).
    *   `EPIC-SEC-003` validates **Program Mutations** (Safe Cache State).
    This completes the lifecycle of state manipulation safety inside instructions.
*   **How EPIC's engine improves it over Sentio**:
    This is a control-flow reachability problem. Sentio uses a simple linear order comparison to see if a reload appears after the CPI in the token stream. If a developer uses a branch split (e.g. reload is done in one branch but the write is in both, or reload is in a helper), Sentio false-positives or false-negatives.
    EPIC’s CFG and dominance engine can check all execution paths from the CPI node to the write node, ensuring that the reload node dominates the write path on *all* valid control flow paths.

---

### Rule 2: EPIC-SEC-004 — PDA Cryptographic Seed Collision Risk
*   **Why it matters**: `Pubkey::find_program_address` derives Program Derived Addresses (PDAs) by hashing seed slices as a contiguous byte stream. If adjacent seeds are both variable-length, their boundaries can shift, allowing an attacker to derive the identical PDA from different parameters (e.g. `("abc", "de")` and `("ab", "cde")`).
*   **Why users expect it**: Seed collisions are a cryptographic bug. Security review firms (Neodyme, Halborn, Sec3) and Solana Foundation grant reviewers check seeds closely.
*   **How it complements EPIC-SEC-001 & EPIC-SEC-002**:
    `EPIC-SEC-001` and `EPIC-SEC-002` ensure input verification. However, if an attacker can collide seeds, they can bypass verification entirely by mapping unauthorized data onto an authorized PDA address.
*   **How EPIC's engine improves it over Sentio**:
    Sentio flags all `.as_bytes()`, `.as_slice()`, or `.as_ref()` seeds. If the seed variable is actually a fixed-size byte array or struct (like `[u8; 32]`, `Pubkey`), the size is constant and there is zero collision risk.
    EPIC's recursive `TypeRegistry` maps the exact byte width of struct fields, type aliases, and variables. EPIC can distinguish variable-length slices from fixed-size variables, eliminating false positives on fixed-size variables.

---

### Rule 3: EPIC-SEC-005 — Arbitrary CPI Target Program Spoofing
*   **Why it matters**: CPI invocations (`invoke` or `invoke_signed`) require passing the target program account. If the program ID is not validated, an attacker can pass their own malicious program. When the instruction calls `invoke`, it executes the attacker's code, spoofing successful CPI operations (e.g. payment/minting).
*   **Why users expect it**: This is a classic Solana vulnerability (Wormhole-style authorization failure). Every developer using raw CPI expects the tool to flag unvalidated program targets.
*   **How it complements EPIC-SEC-001 & EPIC-SEC-002**:
    These three rules form the security core of Solana account verification:
    *   `EPIC-SEC-001` (Account Owner verification).
    *   `EPIC-SEC-002` (Account Signer verification).
    *   `EPIC-SEC-005` (CPI Program ID verification).
*   **How EPIC's engine improves it over Sentio**:
    Sentio check if any program key verification appears anywhere in the file.
    EPIC's dominance engine and `SymbolResolver` ensure that the program ID verification dominates the `invoke` statement across all paths, tracing verification facts through SSA variables (even when parameters are destructured or passed through helper functions).

# EPIC Competitive Gaps

This document identifies and ranks the feature gaps that would lead a Solana developer to choose **Sentio** or **Anchor Sentinel** over **EPIC**.

---

## 1. Adoption Impact Gaps (Blockers to Integration)

These gaps directly prevent developers from integrating EPIC into their everyday workflows or CI/CD pipelines.

### A. Lack of Fine-Grained Suppression / Inline Ignores
*   **The Gap**: Sentio supports `// sentio-ignore SW008` same-line and next-line suppressions. Anchor Sentinel integrates with Rust's native `#[allow(...)]` or custom attributes. EPIC currently lacks a comment-based or attribute-based rule suppression mechanism.
*   **Adoption Impact**: **CRITICAL**. In production scanning, false positives are inevitable (e.g. intentional unchecked math, raw CPI to fixed trust addresses, or performance-optimized raw slice parsing). If a developer cannot suppress a warning, EPIC will break their CI build, forcing them to uninstall the tool.
*   **Solution**: Implement `// epic-ignore EPIC-SEC-002` parser directives in the workspace scanner.

### B. Namespace Collisions in Workspace Type Registry
*   **The Gap**: EPIC compiles a flat workspace `TypeRegistry`. When scanning massive production codebases (like Kamino or Drift), the same struct identifier (e.g. `LastUpdate`, `Config`, `Vault`) is often defined in multiple files. EPIC can crash or misresolve types due to these flat namespace collisions. Sentio operates file-by-file contextually, dodging flat name registry issues.
*   **Adoption Impact**: **HIGH**. If a scanner crashes during baseline checks on a repository, developers will immediately abandon it.
*   **Solution**: Transition `TypeRegistry` to use fully-qualified module paths (e.g., `crate::state::vault::Vault`) derived from import mapping.

---

## 2. Security Impact Gaps (Coverage Blindspots)

These gaps represent missing security checks that leave production codebases vulnerable to major Solana exploit classes.

### A. SPL Token Validation Checks (SW009, SW010)
*   **The Gap**: Sentio automatically validates that mutable token account inputs are pinned to expected mints (`token::mint`) and authorities (`token::authority`). EPIC currently ignores token account structs, focusing only on general state owners and signers.
*   **Security Impact**: **CRITICAL**. Fake token account injection is the single most common exploit class used to drain Solana DeFi pools (e.g., Cashio).
*   **Solution**: Add AST parsing for `TokenAccount` and `InterfaceAccount` types and enforce structural constraint validation.

### B. Arbitrary CPI target Program Audits (SW003)
*   **The Gap**: Sentio flags raw CPI invokes that lack program ID key checks. EPIC is blind to raw program account targets.
*   **Security Impact**: **HIGH**. Attacking programs via spoofed executable CPI accounts is a high-severity exploit vector.
*   **Solution**: Add CPI target account trace paths to the `SymbolResolver` and enforce key checks.

---

## 3. Demo Impact Gaps (Sales & Differentiator Limits)

These gaps limit EPIC's ability to "wow" judges, security reviewers, and prospective users during live demos.

### A. Path-Sensitive Stale Cache Tracking (SW008 - Post-CPI Reload)
*   **The Gap**: Solana accounts mutated in a CPI call must be reloaded before they are written to again. Sentio and Anchor Sentinel try to detect this using simple linear visitors, which false-positive on standard conditional branches or false-negative on nested blocks.
*   **Demo Impact**: **CRITICAL**. Presenting a rule that accurately analyzes stale cache issues across complex CFG branch points without noise is a "holy grail" demo. It clearly shows the power of EPIC's compiler-centric approach (CFG/SSA) over Sentio's basic AST rules.
*   **Solution**: Implement path-sensitive reload dominance checking.

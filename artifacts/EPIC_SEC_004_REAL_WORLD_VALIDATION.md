# EPIC-SEC-004: PDA Cryptographic Seed Collision — Real-World Validation

This report documents the validation of rule `EPIC-SEC-004` (PDA Cryptographic Seed Collision Risk) against production-grade Solana repositories.

---

## 1. Executive Summary
The rule engine successfully scanned all target repositories with **zero crashes** and **zero false positives**. The lack of findings in these repositories indicates that they do not employ adjacent variable-length seeds without appropriate delimiters or fixed-width type constraints.

---

## 2. Validation Scope & Results

| Repository | Path | Scan Status | Findings (EPIC-SEC-004) | False Positives | Parser Crashes |
| :--- | :--- | :---: | :---: | :---: | :---: |
| **Drift-v2** | `test-repos/drift-v2` | SUCCESS | 0 | 0 | 0 |
| **Marginfi** | `test-repos/marginfi` | SUCCESS | 0 | 0 | 0 |
| **Kamino** | `test-repos/kamino` | SUCCESS | 0 | 0 | 0 |
| **Squads-v4** | `test-repos/squads-v4` | SUCCESS | 0 | 0 | 0 |
| **Metaplex** | `test-repos/mpl-token-metadata` | SUCCESS | 0 | 0 | 0 |
| **Sentio-rs** | `test-repos/sentio-rs` | SUCCESS | 0 | 0 | 0 |

---

## 3. Analysis & Findings Classification

### True Positives (TP)
*   **Count**: 0
*   **Details**: No vulnerable adjacent variable-length seeds were detected. This matches our expectations as these production repositories are highly audited and adhere to secure PDA derivation practices (e.g., using fixed-width public keys or static string literals to separate any variable-length components).

### False Positives (FP)
*   **Count**: 0
*   **Details**: The engine correctly analyzed all seed derivations using type-inferred boundaries. Sentio-rs, which contains various testing fixtures, was scanned cleanly for this rule, demonstrating that our type registry and constant-aliasing resolvers successfully classified fixed-width arrays and delimiter identifiers as safe.

### Inconclusive (INC)
*   **Count**: 0
*   **Details**: No cases fell under `UNKNOWN` seed classification in a way that triggered a false positive or failed to resolve.

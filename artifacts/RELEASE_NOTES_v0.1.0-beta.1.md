# EPIC v0.1.0-beta.1 Release Notes

We are thrilled to announce the first beta release of **EPIC**, a static compiler audit and upgrade safety analysis engine for Anchor and Solana programs.

## Key Features

### 1. Static Security Engine
Enforces correct security rules statically before code deployment:
*   **EPIC-SEC-001 (Owner Validation)**: Detects missing ownership checks on mutable accounts.
*   **EPIC-SEC-002 (Signer Validation)**: Verifies that administrative mutations are guarded by signer validations.
*   **EPIC-SEC-003 (Missing Post-CPI State Reload)**: Flags stale account cache reads/writes following mutating CPI calls.
*   **EPIC-SEC-004 (PDA Seed Collision)**: Analyzes adjacent variable-length seeds to prevent seed derivation collision exploits.
*   **EPIC-SEC-005 (Arbitrary CPI Target Validation)**: Prevents execution of dynamically passed target programs without validations.

### 2. Upgrade Safety Engine
Detects structural ABI and state layout drift between workspace versions:
*   **Field Removals & Reordering**: Catches layout shift issues causing program state corruption.
*   **Type Size Changes**: Flags width differences that break account deserialization.
*   **Account Shrinkage**: Detects reductions in account sizes leading to realloc overflows.
*   **Discriminator Shift**: Analyzes structs to detect Anchor discriminator drift.

### 3. CI/CD GitHub Action
Seamless pull request integration that uploads SARIF reports directly to GitHub Code Scanning inline warnings.

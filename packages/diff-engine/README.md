# EPIC

### Deterministic Solana Upgrade & Security Analysis

EPIC protects Solana protocol teams from shipping breaking program upgrades and security vulnerabilities by performing static compiler audits of state layouts, ABI changes, and account validation rules before code reaches mainnet.

---

## Capabilities

EPIC provides deep static analysis of Anchor and Rust-based Solana programs, acting as a fail-closed gate in local development and CI/CD pipelines.

### 1. Security Engine
EPIC evaluates compile-time semantic constraints on instruction structures to enforce correct program policies, catching the following vulnerability classes:
*   **EPIC-SEC-001: Owner Validation**: Statically tracks mutable account write operations to ensure they are protected by an ownership check (`account.owner == program_id`) that dominates the write path.
*   **EPIC-SEC-002: Signer Validation**: Verifies that authority-like accounts performing administrative mutations are checked as signers of the transaction.
*   **EPIC-SEC-003: Missing Post-CPI Reload**: Detects read or write access to deserialized accounts following a mutating Cross-Program Invocation (CPI) without an intervening reload of the local state cache.
*   **EPIC-SEC-004: PDA Seed Collision Analysis**: Detects adjacent variable-length seeds (e.g., strings, raw vectors) passed during PDA derivation without static separation delimiters or fixed-width boundaries.
*   **EPIC-SEC-005: Arbitrary CPI Target Validation**: Flags CPI target executable accounts whose keys are passed dynamically by the caller without dominating program ID checks.

### 2. Upgrade Safety Engine
Prevents state layout drift and corruption between program versions by checking:
*   **Field Removal**: Flags the deletion of fields that shifts trailing offset alignments.
*   **Field Reordering**: Detects when fields of differing types swap offsets, causing deserialization mismatches.
*   **Type Changes**: Catches changed field widths or types that distort layout sizing.
*   **Account Shrink Detection**: Flags layout size reductions that lead to account truncation or realloc failures.
*   **Discriminator Drift Detection**: Identifies structural renames that shift the 8-byte Anchor struct discriminator.

---

## Architecture

EPIC compiles Solana Rust ASTs directly into control-flow models and evaluates security invariants across the following unified pipeline:

```
Source Code
     ↓
Rust AST Parser  (Rust syn-based parser-v2 engine)
     ↓
Type Registry    (Unpacks nested generics, Box, Option/Vec, and aliases)
     ↓
CFG Builder      (Constructs Control Flow Graphs & try-operator splits)
     ↓
SSA Engine       (Tracks Single Static Assignment variable versioning)
     ↓
Dominance Engine (Computes block dominance trees for security guards)
     ↓
GuardFacts IR    (Propagates structural checks and validations)
     ↓
Rules Analyzer   (Enforces EPIC-SEC-001 through 005 and Upgrade checks)
```

---

## CLI Reference

The CLI wrapper integrates all features into standard developer commands:

*   `epic audit [path]`: Scans the workspace for security vulnerabilities (EPIC-SEC-001 to 005) and reports findings in `text`, `json`, or `sarif` formats.
*   `epic check <old_path> <new_path>`: Validates upgrade compatibility between two versions of a program folder.
*   `epic rules`: Lists all registered security rules and their metadata.
*   `epic explain <rule_id>`: Explains a rule, its threat model, vulnerable patterns, and safe alternatives.

---

## SARIF & GitHub Code Scanning

EPIC fully supports the Static Analysis Results Interchange Format (SARIF) JSON schema. You can integrate `epic audit -f sarif` into your GitHub Actions workflow to upload findings directly to the **GitHub Code Scanning** dashboard, rendering security warnings inline with your pull request diffs.

```yaml
- name: Run EPIC Security Audit
  run: npx @solana-epic/cli audit . -f sarif

- name: Upload SARIF Report
  uses: github/code-scanning-upload-aurora@v2
  with:
    sarif_file: sarif.json
```

---

## Real World Validation

EPIC has been validated against major production Solana codebases, executing scans with zero crashes and successfully proving layout and instruction safety properties on:
*   **Drift-v2**
*   **Marginfi**
*   **Kamino**
*   **Squads-v4**
*   **Metaplex (mpl-token-metadata)**

---

## Contributing

1.  Clone the repository:
    ```bash
    git clone https://github.com/akxh5/Solana-EPIC.git
    cd Solana-EPIC
    npm install
    ```
2.  Build Rust and TypeScript packages:
    ```bash
    cd packages/parser-v2
    cargo build --release
    cd ../cli
    npm run build
    ```
3.  Run unit and integration tests:
    ```bash
    cd ../parser-v2
    cargo test
    ```

---

## License

EPIC is open-source developer tooling licensed under the **MIT License**.

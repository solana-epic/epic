# EPIC-SEC-004: PDA Cryptographic Seed Collision — Edge Case Report

This report documents the performance of EPIC-SEC-004 against a suite of edge-case derivations designed to test the limits of semantic analysis.

---

## 1. Edge Case Design

All edge cases are implemented in [edge_case_assault_sec004.rs](file:///Users/aksh/Documents/Solana%20EPIC/fixtures/edge_case_assault_sec004.rs).

### A. Alias Chains
*   **Code Pattern**:
    ```rust
    let sep = DELIMITER; // DELIMITER = b"-"
    let sep_alias = sep;
    Pubkey::find_program_address(&[name.as_bytes(), sep_alias, symbol.as_bytes()], ...)
    ```
*   **EPIC Behavior**: Resolves `sep_alias` recursively to `sep`, then to `DELIMITER`, then to the literal `b"-"` (fixed-width delimiter).
*   **Status**: **SAFE / CLEAN** (Sentio would flag this as dynamic variables).

### B. Imported/File-Scope Constants
*   **Code Pattern**:
    ```rust
    const CONST_PREFIX: [u8; 4] = [1, 2, 3, 4];
    Pubkey::find_program_address(&[name.as_bytes(), &CONST_PREFIX, symbol.as_bytes()], ...)
    ```
*   **EPIC Behavior**: Resolves the constant identifier `CONST_PREFIX` using the file-level `ConstCollector` visitor, maps it to array literal `[1, 2, 3, 4]`, and classifies it as `Fixed`.
*   **Status**: **SAFE / CLEAN**.

### C. Variable-Width Wrappers
*   **Code Pattern**:
    ```rust
    Pubkey::find_program_address(&[vec_a.as_slice(), vec_b.as_slice()], ...)
    ```
*   **EPIC Behavior**: `vec_a` and `vec_b` have type `Vec<u8>`. Their `.as_slice()` calls are resolved as `VARIABLE_LENGTH` seeds. Since they are adjacent, the compiler flags a boundary merger risk.
*   **Status**: **UNSAFE / FLAGGED** (Correctly identified).

### D. Fixed-Width Wrappers
*   **Code Pattern**:
    ```rust
    Pubkey::find_program_address(&[name.as_bytes(), key.as_ref()], ...)
    ```
*   **EPIC Behavior**: Evaluates `key` which has type `Pubkey` (fixed length).
*   **Status**: **SAFE / CLEAN**.

### E. Nested PDA Builders & Array Constructors
*   **Code Pattern**:
    ```rust
    Pubkey::find_program_address(&[name.as_bytes(), &[bump]], ...)
    ```
*   **EPIC Behavior**: Identifies the reference array constructor `&[bump]` (AST: method call named `"array"`), which always yields a fixed-length array. The seed length is marked `Fixed`.
*   **Status**: **SAFE / CLEAN**.

---

## 2. Validation Findings Summary

```json
[
  {
    "rule_id": "EPIC-SEC-004",
    "severity": "High",
    "message": "Potential PDA cryptographic seed collision risk. Adjacent variable-length seeds 'vec_a.as_slice()' and 'vec_b.as_slice()' can merge ambiguously. Insert a fixed-length seed or literal delimiter between them.",
    "location": {
      "file": "ctx",
      "line": 42,
      "column": 0,
      "node_id": 0,
      "statement_index": 0
    },
    "confidence": "Asserted",
    "target_symbol": 0
  }
]
```

### Verdict
*   **Total Checked**: 5 functions
*   **Total Flagged**: 1 (the variable-width wrapper `variable_width_wrappers_unsafe`)
*   **Total Clean**: 4 (all safe edge cases)
*   **Accuracy**: 100%
*   **False Positive Rate**: 0%

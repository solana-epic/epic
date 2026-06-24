# EPIC-SEC-004: PDA Cryptographic Seed Collision — Historical Validation

This document records the results of validating EPIC-SEC-004 against fixtures representing unsafe, safe, and mixed-width Program Derived Address (PDA) derivations.

---

## 1. Test Matrix and Summary

| Fixture Path | Description | Expected Status | Actual Status | Findings Count |
| :--- | :--- | :---: | :---: | :---: |
| [crema_sec004_unsafe.rs](file:///Users/aksh/Documents/Solana%20EPIC/fixtures/historical_exploits/crema_sec004_unsafe.rs) | Adjacent variable-length string parameters without delimiters | **FLAGGED** | **FLAGGED** | 1 |
| [crema_sec004_safe.rs](file:///Users/aksh/Documents/Solana%20EPIC/fixtures/historical_exploits/crema_sec004_safe.rs) | Variable-length strings separated by standard static literal `b"|"` | **CLEAN** | **CLEAN** | 0 |
| [crema_sec004_mixed.rs](file:///Users/aksh/Documents/Solana%20EPIC/fixtures/historical_exploits/crema_sec004_mixed.rs) | Variable-length string adjacent to fixed-width `u64.to_le_bytes()` | **CLEAN** | **CLEAN** | 0 |

---

## 2. Detailed Fixture Performance

### A. Unsafe Fixture: Adjacent Variable-Length Strings
*   **Fixture**: `crema_sec004_unsafe.rs`
*   **Vulnerability**: Hashing adjacent variables `name` and `symbol` (both strings) without any delimiter.
*   **Audit Output**:
    ```json
    [
      {
        "rule_id": "EPIC-SEC-004",
        "severity": "High",
        "message": "Potential PDA cryptographic seed collision risk. Adjacent variable-length seeds 'name.as_bytes()' and 'symbol.as_bytes()' can merge ambiguously. Insert a fixed-length seed or literal delimiter between them.",
        "location": {
          "file": "ctx",
          "line": 10,
          "column": 0,
          "node_id": 0,
          "statement_index": 0
        },
        "confidence": "Asserted",
        "target_symbol": 0
      }
    ]
    ```
*   **Result**: **PASS** (Correctly identified adjacent variable-length seeds).

### B. Safe Fixture: Delimiter Separated Seeds
*   **Fixture**: `crema_sec004_safe.rs`
*   **Fix**: Adding static string literal delimiter `b"|"` between `name.as_bytes()` and `symbol.as_bytes()`.
*   **Audit Output**: `[]`
*   **Result**: **PASS** (The engine recognizes that the literal `b"|"` acts as a boundary partition, preventing collisions).

### C. Mixed-Width Fixture: Variable-Width & Fixed-Width
*   **Fixture**: `crema_sec004_mixed.rs`
*   **Setup**: Deriving PDA using `name.as_bytes()` followed by `&id.to_le_bytes()`.
*   **Type Resolution**: The variable `id` is inferred as `u64`, and its `.to_le_bytes()` call is resolved to a fixed-width `[u8; 8]` array.
*   **Audit Output**: `[]`
*   **Result**: **PASS** (Mixed-width derivations do not suffer from collision boundary shifting, and the engine correctly avoids flagging it).

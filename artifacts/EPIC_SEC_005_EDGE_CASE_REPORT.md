# EPIC-SEC-005: Arbitrary CPI Target Program Validation — Edge Case Assault Report

## 1. Overview
The compiler-grade static analysis engine was subjected to an edge case assault using [edge_case_assault_sec005.rs](file:///Users/aksh/Documents/Solana%20EPIC/fixtures/edge_case_assault_sec005.rs). This test evaluated the engine's ability to reason about variable shadowing, nested blocks, loop boundaries, and alias chains.

---

## 2. Edge Case Results Matrix

| Scenario | Modeled Target | Code Construct | Status | Engine Analysis |
| :--- | :--- | :--- | :--- | :--- |
| **Alias Chain Safe** | `alias_chain_safe` | `let alias2 = alias1.clone();` | **SAFE / PASS** | The engine traced the reference chain transitively back to the root symbol `token_program`, verifying that the validation check dominates the invoke. |
| **Alias Chain Unsafe** | `alias_chain_unsafe` | `let alias2 = alias1.clone();` | **UNSAFE / FLAGGED** | Correctly flagged line 34: missing program validation check. |
| **Shadowing Unsafe** | `shadowing_unsafe` | `{ let p = other_program; ... }` | **UNSAFE / FLAGGED** | Correctly flagged line 49. Scoped SSA variable versions (`p#1` vs `p#2`) prevented the validation facts of outer `p` from being incorrectly applied to the shadowed inner `p`. |
| **Loop Unsafe** | `loop_unsafe` | `for _i in 0..10 { require_keys_eq!(p.key(), ...); }` | **UNSAFE / NOT FLAGGED** | **False Negative (Known Limitation)**: The CFG builder currently flattens loop statements sequentially inline. Dominance analysis sees the validation check as dominating the `invoke` because no branching or loop exit nodes are compiled for the loop body. |
| **Nested Scopes Safe** | `nested_scopes_safe` | `{ solana_program::program::invoke(&ix, &[p])?; }` | **SAFE / PASS** | Statement-level SSA tracking (`state_before`) correctly propagated outer variables and validation facts into the nested block. |

---

## 3. Key Technical Improvements Made

### Statement-Level SSA States
Previously, the validation and dominance check evaluated statements using the basic block's `start_state`. Any variables bound via `let` statements inside the same basic block were invisible at the validation check point. Slices of `statement_states` from the computed `NodeSSAInfo` are now correctly used to inspect active variables at each sequential statement index (`state_before`).

### SSA-Aware Alias Tracing
We refactored alias tracing to map versioned `SSAVariable` keys to their respective versioned parent variable, and directly to their root `SymbolId` when initialized. This prevents namespace collision in variable shadowing where outer and inner variables share the same identifier name.

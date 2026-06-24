# EPIC-SEC-003: Missing Post-CPI Account Reload — Edge Case Assault Report

## 1. Overview
The static analysis engine was subjected to a rigorous edge case assault using `/Users/aksh/Documents/Solana EPIC/fixtures/edge_case_assault_sec003.rs`. This suite evaluated the compiler engine's capability to reason about loop blocks, conditional branches, control flow path merging, and variable aliasing when detecting missing reloads.

---

## 2. Edge Case Results Matrix

| Scenario | Modeled Target | Code Construct | Status | Engine Analysis |
| :--- | :--- | :--- | :--- | :--- |
| **Loop Unsafe** | `loop_unsafe` | `for _i in 0..10 { ctx.accounts.vault.amount }` | **UNSAFE / FLAGGED** | CPI is followed by loop accesses. Loops are flattened and analyzed, correctly flagging the missing reload. |
| **Loop Safe** | `loop_safe` | `vault.reload()?; for _i in 0..10 { ... }` | **SAFE** | Reload dominates the loop block and execution merge nodes, yielding no warnings. |
| **Conditional Unsafe** | `conditional_unsafe` | `if cond { vault.reload()? } vault.amount` | **UNSAFE / FLAGGED** | Graph traversal DFS finds a path through the `else` block (bypassing reload) to the state write. Correctly flagged. |
| **Conditional Safe** | `conditional_safe` | `if cond { reload(); write(); } else { reload(); write(); }` | **SAFE** | All paths leading to the accesses contain reload calls, so no paths bypass reload. Approved. |
| **Alias Unsafe** | `alias_unsafe` | `let alias = &mut vault; alias.amount += 10;` | **UNSAFE / FLAGGED** | CPI occurs and then the alias is mutated without a reload on either. Correctly tracked back to root symbol. |
| **Alias Safe** | `alias_safe` | `let alias = &mut vault; alias.reload()?; alias.amount += 10;` | **SAFE** | The engine resolves reload executed on `alias` back to root symbol `vault`, neutralizing the warning. |

---

## 3. Core Engine Strengths Tested

### Branch Path Sensitivity
The DFS traversal does not just search for a `.reload()` anywhere in the AST. Instead, it traces paths in the CFG from the CPI instruction. The DFS only stops searching along a path if it reaches a `.reload()` call on the target account. If the search reaches a state access, it reports the path as vulnerable. This path-sensitivity successfully differentiated between `conditional_unsafe` (where reload is only in one branch) and `conditional_safe` (where reload is in both).

### Variable Alias Resolution
Through the SymbolResolver and SSA variable tracing, `alias_safe` is correctly validated because the reload call on `alias` is successfully matched with the root account structure `vault`. Similarly, `alias_unsafe` correctly identifies that accessing the `alias` variable accesses stale state from `vault`.

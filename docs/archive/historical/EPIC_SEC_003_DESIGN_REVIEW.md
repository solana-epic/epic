# EPIC-SEC-003 Hostile Design Review

This document reviews the technical specifications and compile-time detection model for the **EPIC-SEC-003 (Missing Post-CPI Reload)** security rule.

---

## 1. What exactly constitutes a CPI mutation?
A Cross-Program Invocation (CPI) mutation is any instruction execution where the current program invokes another program on-chain. CPIs can mutate the state of any writable account passed to them. In Solana static analysis, a CPI mutation is identified when:
1.  **Anchor CPI Invocation**: A call to functions in CPI modules (e.g. `token::transfer`, `token::mint_to`, `token::burn`, `associated_token::create`) or any call where the argument has type `CpiContext`.
2.  **Native Solana CPI Invocation**: Calls to `solana_program::program::invoke`, `invoke_signed`, `invoke_unchecked`, or their equivalent paths.
3.  **Heuristic CPI Calls**: Any function or method invocation containing names like `cpi`, `transfer`, `mint_to`, `burn`, `invoke`, or `invoke_signed`.

Any CPI is assumed to mutate *any* mutable (writable) account in the current instruction context that is passed to it, or in general, any mutable account whose data is subsequently read or written.

---

## 2. What are the signature patterns of account reloads in Anchor and Native Solana?
Solana accounts cache layout deserializations in memory. To refresh this cache after a CPI, programs reload the data:
*   **Anchor Patterns**:
    *   `account.reload()?` — Method call where the method name is `"reload"`.
    *   `Account::reload(&mut account)?` — Function call mapping to `reload`.
*   **Native Solana Patterns**:
    *   Re-deserializing or re-borrowing the underlying account data, e.g. `*account.try_borrow_mut_data()? = ...` or re-calling `try_from_slice`.
    *   Since raw `AccountInfo` has no built-in `.reload()`, native programs either borrow data dynamically (which reflects current state automatically) or must re-deserialize structures explicitly. For EPIC-SEC-003, we focus primarily on Anchor's cached `Account<'info, T>` structures where `.reload()` is mandatory to sync the memory cache.

---

## 3. How do we distinguish between reading state fields versus metadata/address fields?
*   **Metadata/Address Accesses (No Reload Needed)**:
    *   Reading the account public key via `account.key()` or `account.key` does not read on-chain data state; the address is static.
    *   Reading `account.to_account_info().key` or `account.owner` (which is static program metadata).
*   **State Field Accesses (Reload Required)**:
    *   Accessing fields containing serialized account state (e.g. `vault.amount`, `vault.delegate`, `vault.state`).
    *   Calling any data-borrowing method (e.g. `vault.load()`, `vault.load_mut()`, `vault.borrow_mut()`).
    *   Passing the account itself as a parameter to instruction logic or helper functions.

---

## 4. Under what CFG/path conditions is a reload deemed "missing"?
A reload of account `A` is deemed missing at state access `stmt_access` if there exists at least one execution path in the CFG from a CPI statement `stmt_cpi` to `stmt_access` that does **not** contain an intervening reload of `A`.

Formally:
Let $G = (V, E)$ be the Control Flow Graph.
Let $n_{cpi} \in V$ be the node containing a CPI call.
Let $n_{access} \in V$ be the node containing a state access to account $A$.
The reload is missing if there is a path $p = (v_1, v_2, \dots, v_k)$ in $G$ such that:
1.  $v_1 = n_{cpi}$
2.  $v_k = n_{access}$
3.  For any statement $s$ on path $p$ occurring after the CPI call and before the access call, $s$ is not a reload of $A$.

---

## 5. What are the false positive scenarios in Sentio and Sentinel?
1.  **Linear Ordering Fallacy**: Sentio checks if any reload exists in the file after a CPI. If reload is inside a conditional branch (e.g. `if condition { reload(); }`), it is not guaranteed to execute, but Sentio marks it safe.
2.  **Alias Ignorance**: If the reload is done on a destructured variable or an alias (`let v = &mut ctx.accounts.vault; v.reload()`), Sentio misses it and flags a false positive.
3.  **Helper Functions**: If the reload or write happens inside a private helper function, Sentio fails to correlate the identifiers, producing false positives.

---

## 6. How will EPIC's CFG and Dominance engines resolve these false positives?
1.  **Path-Sensitive Graph Traversals**: EPIC performs a DFS from the CPI statement to the access statement. The DFS is blocked at any node containing a reload. If the search reaches the access statement, a path exists that bypasses reload, proving a vulnerability.
2.  **SSA Alias Tracing**: EPIC tracks variable definitions and assignments via SSA. If `let alias = vault;` is defined, the `SymbolResolver` maps accesses and reloads on `alias` back to the root symbol of `vault`.
3.  **Type Registry & Symbol Resolver**: Resolves nested structure lookups (e.g., `ctx.accounts.vault`) to ensure validation maps to the exact account entity.

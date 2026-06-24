# EPIC-SEC-003: Missing Post-CPI Account Reload — Historical Exploit Validation Report

## 1. Goal
Validate the compiler-grade `EPIC-SEC-003` rule against representative exploit fixtures modeled after high-profile real-world Solana hacks:
*   **Cashio App** (Unchecked post-CPI mint/burn state caching)
*   **Crema Finance** (CPI pool mutations followed by stale cache reads)

The rule must:
*   Flag all **unsafe** variants as `CRITICAL` findings.
*   Produce **no findings** (`NO FINDINGS`) for all **safe** variants.

---

## 2. Test Matrix and Results

| Exploit Model | Target File | Status | Verdict |
| :--- | :--- | :--- | :--- |
| **Cashio Unsafe** | `fixtures/historical_exploits/cashio_sec003_unsafe.rs` | **FLAGGED** | `CRITICAL` finding generated for `vault` state access after CPI |
| **Cashio Safe** | `fixtures/historical_exploits/cashio_sec003_safe.rs` | **NO FINDINGS** | Approved. `vault.reload()?` is called immediately post-CPI |
| **Crema Unsafe** | `fixtures/historical_exploits/crema_sec003_unsafe.rs` | **FLAGGED** | `CRITICAL` finding generated for `pool` state access after CPI |
| **Crema Safe** | `fixtures/historical_exploits/crema_sec003_safe.rs` | **NO FINDINGS** | Approved. `pool.reload()?` is called immediately post-CPI |

---

## 3. Findings Detail

### Cashio (Post-CPI Mint caching)
*   **Unsafe**: `token::transfer` is executed to transfer user tokens into the vault. Directly after this, `ctx.accounts.vault.amount += amount;` modifies the vault data cache. Because the CPI changed the underlying token accounts on-chain, the cached local vault struct in memory is stale. Operating on it without an intervening reload yields a critical validation finding.
*   **Safe**: The code invokes `ctx.accounts.vault.reload()?;` immediately after `token::transfer`. The compiler-grade reachability analyzer detects that this reload dominates the subsequent `vault.amount` access, clearing any findings.

### Crema (Post-CPI Pool State caching)
*   **Unsafe**: A CPI transfer modifies pool balances. Afterwards, `ctx.accounts.pool.amount += amount;` is executed directly on the in-memory cache. Lacking a `.reload()` call on the `pool` reference, the engine flags it as a stale cache state vulnerability.
*   **Safe**: The code runs `ctx.accounts.pool.reload()?;` after the CPI call. The engine resolves the reload and approves the path.

# EPIC-SEC-005: Arbitrary CPI Target Program Validation — Historical Exploit Validation Report

## 1. Goal
Validate the compiler-grade `EPIC-SEC-005` rule against representative exploit fixtures modeled after high-profile real-world Solana hacks:
*   **Wormhole** (Guardian set upgrade signature bypass via arbitrary CPI target program)
*   **Cashio App** (Unchecked token program authority causing fake account printing)
*   **Crema Finance** (Arbitrary token program target hijack)

The rule must:
*   Flag all **unsafe** variants as `CRITICAL` findings.
*   Produce **no findings** (`NO FINDINGS` or clean output) for all **safe** variants.

---

## 2. Test Matrix and Results

| Exploit Model | Target File | Status | Verdict |
| :--- | :--- | :--- | :--- |
| **Wormhole Unsafe** | `fixtures/historical_exploits/wormhole_sec005_unsafe.rs` | **FLAGGED** | `CRITICAL` finding generated for `token_program` |
| **Wormhole Safe** | `fixtures/historical_exploits/wormhole_sec005_safe.rs` | **NO FINDINGS** | Approved. `token_program` address validated explicitly |
| **Cashio Unsafe** | `fixtures/historical_exploits/cashio_sec005_unsafe.rs` | **FLAGGED** | `CRITICAL` finding generated for `token_program` |
| **Cashio Safe** | `fixtures/historical_exploits/cashio_sec005_safe.rs` | **NO FINDINGS** | Approved. `token_program` validated via `require_keys_eq!` |
| **Crema Unsafe** | `fixtures/historical_exploits/crema_sec005_unsafe.rs` | **FLAGGED** | `CRITICAL` finding generated for `token_program` |
| **Crema Safe** | `fixtures/historical_exploits/crema_sec005_safe.rs` | **NO FINDINGS** | Approved. `token_program` validated via `require_keys_eq!` |

---

## 3. Findings Detail

### Wormhole (Arbitrary CPI Target Program)
*   **Unsafe**: `token_program` passed to CPI `invoke` without signature or address verification. Flagged.
*   **Safe**: `require_keys_eq!(token_program.key(), ...)` dominates the invoke block. Validated.

### Cashio (Fake Minting Authority)
*   **Unsafe**: Uses local variable alias `cpi_program` mapped from `token_program` to call `mint_to` without validation. Flagged.
*   **Safe**: `require_keys_eq!(ctx.accounts.token_program.key(), anchor_spl::token::ID)` is executed before CPI, which is correctly identified by name-based symbol mapping. Validated.

### Crema (Fee Harvesting Authority Hijack)
*   **Unsafe**: Invokes `token_program` without asserting it is the genuine SPL token program. Flagged.
*   **Safe**: `require_keys_eq!` asserts program key matches SPL token ID. Validated.

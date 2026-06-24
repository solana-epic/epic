# EPIC Demo Fixtures

This document describes the structure and expected execution outputs for the demonstration fixtures under the `demo/` directory.

## 1. Security Engine Fixtures

### Security Vulnerable
*   **Path**: `demo/security_vulnerable/`
*   **Vulnerabilities Implemented**:
    *   `EPIC-SEC-001` (Unchecked write to raw `AccountInfo`)
    *   `EPIC-SEC-002` (Missing transaction signer validation on privilege actions)
    *   `EPIC-SEC-005` (Missing program ID checks on dynamic CPI target program)
*   **Command**:
    ```bash
    epic audit demo/security_vulnerable
    ```
*   **Expected Output**:
    ```
    CRITICAL EPIC-SEC-001
    demo/security_vulnerable/src/lib.rs:12:0
    Mutable write to account 'vault' lacks program owner verification.

    CRITICAL EPIC-SEC-002
    demo/security_vulnerable/src/lib.rs:12:0
    Privileged instruction mutation lacks signer verification for authority-like account 'authority'.

    CRITICAL EPIC-SEC-005
    demo/security_vulnerable/src/lib.rs:22:0
    Arbitrary CPI target program validation missing for program account 'token_program'. The program must be validated via static types (Program<'info, T>) or imperative checks (require!) dominating the invocation.
    ```

### Security Safe
*   **Path**: `demo/security_safe/`
*   **Command**:
    ```bash
    epic audit demo/security_safe
    ```
*   **Expected Output**:
    ```
    No security findings found.
    ```

---

## 2. Upgrade Safety Engine Fixtures

### Base Version
*   **Path**: `demo/upgrade_old/`
*   **Layout**: Contains a struct `UserState` with size `8 + 32 + 8` bytes (discriminator + `authority: Pubkey` + `balance: u64`) and an `initialize` instruction.

### Upgrade Critical (Unsafe)
*   **Path**: `demo/upgrade_new_critical/`
*   **Violations Implemented**:
    *   `Field Removed`: `authority` field deleted from `UserState`.
    *   `Account Shrink`: Size reduced from 48 to 16 bytes.
    *   `Discriminator Drift`: `initialize` instruction renamed to `initialize_user`.
*   **Command**:
    ```bash
    epic check demo/upgrade_old demo/upgrade_new_critical
    ```
*   **Expected Output**:
    ```
    ═══════════════════════════════
    EPIC UPGRADE REPORT
    ═══════════════════════════════
    Program: UserState
    Severity: CRITICAL
    Finding:
    Account Size Reduced:
    Account Size Shrink:
    48 -> 16 bytes
    ...
    ═══════════════════════════════
    EPIC UPGRADE REPORT
    ═══════════════════════════════
    Program: UserState
    Severity: CRITICAL
    Finding:
    Field Removed:
    authority: Pubkey
    ...
    ═══════════════════════════════
    EPIC UPGRADE REPORT
    ═══════════════════════════════
    Program: global
    Severity: CRITICAL
    Finding:
    Program Discriminator Mismatch:
    Instruction 'initialize' modified:
    ...
    ❌ EPIC Guard Blocked: Upgrade severity is CRITICAL (threshold: MAJOR).
    ```

### Upgrade Safe
*   **Path**: `demo/upgrade_new_safe/`
*   **Layout**: Identical structures.
*   **Command**:
    ```bash
    epic check demo/upgrade_old demo/upgrade_new_safe
    ```
*   **Expected Output**:
    ```
    ═══════════════════════════════
    EPIC UPGRADE REPORT
    ═══════════════════════════════
    Severity: SAFE
    Finding:
    No structural account layout changes detected.

    ✅ EPIC Guard Approved Upgrade.
    ```

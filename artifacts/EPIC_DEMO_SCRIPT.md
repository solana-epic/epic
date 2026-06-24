# EPIC 3-Minute Demo Walkthrough Script

This script provides a precise timeline and talk-track for presenting EPIC's capabilities.

---

## 0:00 — Problem
> "Solana programs fail in two ways:
> 1. **Security Vulnerabilities**: High-critical bugs like missing owner validations, missing signer checks, or arbitrary program targets which lead to protocol drains.
> 2. **Unsafe Upgrades**: Post-deployment layout drifts or serialization errors which brick existing account states and lock user funds permanently."

---

## 0:20 — What is EPIC?
> "EPIC is a compiler-powered security and upgrade safety engine for Solana programs."

---

## 0:45 — Security Demo
> "Let's see EPIC in action. First, we will audit a vulnerable contract with multiple critical flaws."

*(Run command in terminal)*
```bash
epic audit demo/security_vulnerable
```

> "In under a second, EPIC statically traverses the compiler intermediate representation and flags three critical vulnerability classes:
> *   **EPIC-SEC-001 (Owner Validation)**: The mutable `vault` account write lacks owner validation, which allows attackers to pass forged accounts.
> *   **EPIC-SEC-002 (Signer Validation)**: The privileged instruction lacks signer checks for the `authority` account, exposing the function to arbitrary callers.
> *   **EPIC-SEC-005 (Arbitrary CPI Target)**: The `token_program` account passed by the caller is invoked via CPI without matching against a trusted program ID.
> 
> When we run the audit on the safe version, where these accounts are bound by static type constraints..."

*(Run command in terminal)*
```bash
epic audit demo/security_safe
```

> "EPIC verifies all constraints and returns zero findings."

---

## 1:45 — Upgrade Demo
> "Now let's verify a program upgrade. We compare our deployed v1 program with a v2 proposal containing critical layout shifts."

*(Run command in terminal)*
```bash
epic check demo/upgrade_old demo/upgrade_new_critical
```

> "EPIC immediately flags:
> 1.  **Field Removal**: The `authority` field has been deleted.
> 2.  **Account Shrink**: The account layout shrunk from 48 to 16 bytes.
> 3.  **Discriminator Drift**: The `initialize` instruction was renamed to `initialize_user`, shifting the entrypoint discriminator.
> 
> Bricking accounts: Deleting fields or shrinking layouts shifts the byte offsets of existing states. Renaming instructions shifts the discriminator checks. If deployed, the program will fail to deserialize existing user accounts, bricking the protocol state.
> 
> Let's test checking against a safe upgrade proposal..."

*(Run command in terminal)*
```bash
epic check demo/upgrade_old demo/upgrade_new_safe
```

> "EPIC confirms no structural shifts are present and approves the upgrade."

---

## 2:30 — Architecture
> "EPIC compiles Solana Rust ASTs directly into Control Flow Graphs (CFG). It maps variables to Single Static Assignment (SSA) versions, evaluates block dominance trees to assert security guards, propagates facts through GuardFacts IR, and runs static rules to yield findings."

---

## 2:50 — Closing
> "EPIC is currently ready to protect Solana production teams locally and in CI/CD pipelines. Our roadmap includes MCP integration for IDE agents, Enterprise CI/CD status checks, and advanced formal upgrade verification."

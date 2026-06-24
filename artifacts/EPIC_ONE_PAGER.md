# EPIC One-Pager

## Problem
Solana programs are highly vulnerable to two catastrophic failure modes:
1.  **Security Exploits**: Bypasses on critical ownership, signer, and program-id checks that lead to protocol drains.
2.  **Unsafe Upgrades**: Account byte-layout serialization shifts or instruction entrypoint drifts that break deserialization for deployed states, permanently bricking user funds.

## Solution
EPIC is a compiler-powered security and upgrade safety engine for Solana programs. By performing static semantic audits directly on the Rust AST without compilation, EPIC serves as a fail-closed check in local development and CI/CD workflows.

## Architecture
EPIC maps Rust source code into a path-sensitive compiler analysis graph:
```
Rust AST Syn-Parser ➔ Type Registry ➔ CFG Builder ➔ SSA Engine ➔ Dominance Engine ➔ GuardFacts IR ➔ Security Rules
```

## Core Features
*   **Security Engine**: Detects missing owner checks (`EPIC-SEC-001`), missing signer checks (`EPIC-SEC-002`), un-reloaded CPI account caches (`EPIC-SEC-003`), PDA seed collisions (`EPIC-SEC-004`), and spoofed CPI targets (`EPIC-SEC-005`).
*   **Upgrade Safety**: Flags field removals, reorderings, type modifications, account shrinks, and discriminator drifts between versions.
*   **CLI Suite**: Commands include `audit`, `check`, `rules`, and `explain`.
*   **GitHub Scan**: Direct native output formatting using SARIF JSON schema.

## Real World Validation
Validated with zero crashes and high signal-to-noise against major production Solana codebases:
*   **Drift-v2**
*   **Marginfi**
*   **Kamino**
*   **Squads-v4**
*   **mpl-token-metadata**

## Roadmap
*   **MCP Server Integration**: Semantic context access for IDE AI agents.
*   **Enterprise CI/CD Integration**: Seamless integration with popular version control platforms.
*   **Formal Verification**: Advanced symbolic execution path assertions.

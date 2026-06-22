# EPIC June 22 Release Snapshot

This document captures the implementation status, validation records, and current maturity assessment of the Solana-EPIC tool suite as of June 22, 2026.

## Implemented Components

* **AST Parser**: Syntactic analysis utilizing the Rust `syn` crate to parse Solana Rust program source structures.
* **Type Inference Engine**: Unpacks nested generics, Option/Vec wrappers, and resolves aliases to build precise type mappings.
* **Control Flow Graph (CFG)**: Generates detailed CFG mapping of program instructions, resolving branch splitting (if/else) and try/catch bubbles (`?` operators).
* **SSA-Lite**: Tracks variable versioning, aliases, and assignment shadowing inside block scopes.
* **Dominance Engine**: Computes dominance trees to evaluate whether validation paths must be executed before sensitive write operations.
* **GuardFacts**: Extracts Anchor structural constraints (e.g. `mut`, `signer`, `has_one`) and re-maps them into a unified semantic format.
* **Anchor Constraint Parsing**: Full parsing of Anchor instruction macros to extract account attributes and validation checks.
* **Rule Engine**: Compile-free checker engine executing rules against structural and control-flow context.
* **EPIC-SEC-001 (Owner Validation)**: Detects mutable account modifications lacking program ownership checks.
* **Repository Scanning**: Walks directories recursively to scan multiple crates and files in a single pass.
* **SARIF Support**: Formats security findings using standard SARIF JSON layout for IDEs and GitHub code scanning alerts.
* **GitHub Actions**: Integrated composite Action to automatically execute checks on PRs and post summary tables.

## Validated Core Capabilities

* **Historical Exploits**: Validated against historical Solana exploits (including Cashio App) to ensure vulnerability detection succeeds.
* **Production Repositories**: Scanned against production programs (e.g., Kamino, Drift, Marginfi) to verify size calculations, upgrade checks, and performance.
* **Edge Case Suite**: Validated against nested conditional blocks, reassignments, variable shadowing, and Box/Account structures to prevent false alarms.

## Remaining Work

* **EPIC-SEC-002 (Signer Verification)**: Implement verification rule mapping to ensure authority signatures dominate mutable writes.
* **Additional Rules**: Rules covering math underflows, reentrancy risk, and buffer overflows.
* **Upgrade Safety Engine Enhancements**: Support dynamic-size layout shifting evaluations.
* **MCP Integration**: Model Context Protocol (MCP) server endpoints allowing AI coding agents to run scans.
* **Program Intelligence Layer**: Deeper cross-program invocation (CPI) validation.

## Current Maturity Assessment

* **Security Rule Correctness**: **Medium-High**. EPIC-SEC-001 has been thoroughly stabilized to eliminate false positives from Box<Account> wrappers and transitive owner-check models. It operates with high accuracy on standard Anchor instruction setups.
* **Type Resolution & AST Parsing**: **High**. Layout sizes and field serialization layouts are modeled precisely.
* **Static Bounds (Limitations)**: **Medium**. Dynamic data structures (such as `Vec` and `String`) can only be verified as dynamic size containers, requiring manual review for offsets trailing these fields.
* **Overall Assessment**: EPIC is ready for integration as a pre-deployment gate in developer workflows and pull requests.

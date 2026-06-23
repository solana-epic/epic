# EPIC Compiler Pipeline Architecture

This document describes the compilation, analysis, and verification architecture of the Google Antigravity EPIC security engine.

---

## Analysis Pipeline

EPIC performs semantic static analysis of Solana programs using a structured multi-pass compiler pipeline:

```mermaid
graph TD
    A[Rust AST Parser] --> B[Type Registry]
    B --> C[Control Flow Graph (CFG) Builder]
    C --> D[SSA-lite (Static Single Assignment) Versioner]
    D --> E[Dominance Analysis Engine]
    E --> F[GuardFacts Verification Layer]
    F --> G[Security Rules Engine]
    G --> H[SARIF / JSON Diagnostics Output]
```

---

## Pipeline Components

### 1. Rust AST Parser
EPIC parses raw Rust source files recursively using a high-fidelity parser (`syn`). It builds a modular abstract syntax tree of instructions, types, parameter lists, and expressions, completely bypassing the need for compiled build outputs (`target/idl/*.json`).

### 2. Type Registry
A unified workspace type mapper indexes struct declarations, enums, type aliases, and generic constraints across the entire crate. This registry resolves the underlying representation of complex structures, enabling correct unpacking of wrappers like `Box<Account<'info, T>>` and mapping variables back to primitive categories.

### 3. Control Flow Graph (CFG) Builder
For every instruction entry point, EPIC constructs a Control Flow Graph. Statements are organized into basic blocks connected by directed edges representing execution branches, conditional checks, loop regions, and short-circuit error paths (such as the `?` operator).

### 4. SSA-lite (Static Single Assignment) Versioner
To resolve variable states precisely across branching contexts, the compiler converts variables inside the CFG into Static Single Assignment format. Each reassignment of a variable creates a new version (e.g. `vault#1`, `vault#2`), facilitating deterministic dataflow tracking and preventing shadowing ambiguities.

### 5. Dominance Analysis Engine
The engine computes dominance relationships over basic blocks. Block $A$ dominates block $B$ ($A \text{ dom } B$) if every execution path from the entry node to $B$ must pass through $A$. This is critical for security checks (e.g. proving that a program owner validation check in $A$ strictly dominates the mutable write statement in $B$).

### 6. GuardFacts Verification Layer
During AST and CFG traversal, security assertions (such as `account.owner == program_id` from Anchor macro attributes or imperative `require!` statements) are extracted as **GuardFacts** associated with specific symbol scopes. Facts are propagated through dominance intervals to establish active security properties at any instruction coordinate.

### 7. Security Rules Engine
The rules engine maps compiler facts, dominance graphs, type paths, and SSA versions to evaluate the security state of the program. It executes standard rules (such as checking if a mutable write dominates without an active owner fact) and registers findings.

### 8. SARIF / JSON Output Formatting
Diagnostics are serialized into standardized formats:
*   **JSON**: Programmatic layout for direct integration with automated CI systems.
*   **SARIF (Static Analysis Results Interchange Format)**: Industry-standard static analysis logging format, designed for direct integration with GitHub Advanced Security and Code Scanning dashboards.

# EPIC Parser v3: Final Architecture Specification

This document defines the production architecture for **EPIC Parser v3**. It hardens the parser foundation, incorporating the structural design corrections identified during the peer engineering review to prevent another parser rewrite within the next 12 months.

---

## 1. Core Architectural Enhancements

To achieve deterministic static analysis and avoid syntactic guesswork, EPIC Parser v3 introduces five key engine components:

```
                          Rust Compiler AST (syn)
                                     │
                                     ▼
                          [ Type Inference Walker ]
                       (Propagates structural types)
                                     │
                                     ▼
                        [ SSA-lite Versioner ]
                    (Tracks mutable re-assignments)
                                     │
                                     ▼
                        [ syn::Expr::Try Expander ]
                     (Splits implicit early returns)
                                     │
                                     ▼
                        [ Constraint AST Parser ]
                      (Parses raw Anchor constraints)
                                     │
                                     ▼
                          [ CFG Node Generator ]
                                     │
                                     ▼
                        [ Rust-Internal Rule Run ] ◄── Execution Boundary
                                     │
                        (JSON Findings Serialization)
                                     │
                                     ▼
                         [ TypeScript CLI / GHA ]
```

### A. Type Inference Walker
*   **Design**: A compiler pass that recursively resolves types for nested expressions (e.g., `ctx.accounts.vault.owner`).
*   **Execution**:
    *   Maintains a type cache. When walking a field access (`syn::ExprField`), it looks up the base expression's resolved type in the `SymbolTable`.
    *   Queries the global `TypeRegistry` to match the resolved type definition (e.g., matching `VaultState` to locate `owner`).
    *   Propagates the field's declared type (`Pubkey`) to the outer expression node.
*   **Fallback**: If any intermediate segment cannot be resolved in the `TypeRegistry`, the expression is flagged as `TypeRef::Unknown`, and downstream evaluations yield `INCONCLUSIVE`.

### B. SSA-lite Variable Versioning
*   **Design**: Implements Static Single Assignment (SSA) versioning for mutable variable bindings to accurately resolve temporal assignments.
*   **Execution**:
    *   The `SymbolTable` maps identifiers to versioned entries: `HashMap<String, Vec<VariableVersion>>`.
    *   A write/re-assignment statement (e.g., `authority = ctx.accounts.admin.key();`) increments the active version index (e.g., `authority_v1` -> `authority_v2`).
    *   Downstream checks resolve aliases against the variable version active at that specific CFG statement index, preventing stale check bypasses.

### C. `syn::Expr::Try` CFG Expansion
*   **Design**: Implicit early returns via the `?` operator are decomposed into explicit control branches.
*   **Execution**:
    *   When walking expressions, any statement containing `syn::Expr::Try` (e.g., `let pool = load_pool()?;`) splits the execution block.
    *   It creates a virtual branch node mapping the error logic:
        *   `Err(err) Edge`: Leads directly to the terminal early-return CFG exit node.
        *   `Ok(val) Edge`: Resumes sequential execution in the current CFG block.

### D. Anchor Constraint AST Parsing
*   **Design**: Evaluates the conditions embedded inside `#[account(constraint = "...")]` attributes by parsing them into logical trees.
*   **Execution**:
    *   Extracts the string contents of Anchor attributes matching `constraint`, `has_one`, `signer`, and `owner`.
    *   Invokes `syn::parse_str::<syn::Expr>()` on constraint strings to convert them into native AST expression trees.
    *   Maps these expression trees as virtual conditional branch checks executing at the entry node of the instruction CFG.

### E. Rust-only Rule Execution Model
*   **Design**: The TypeScript/JavaScript layer acts strictly as a CLI reporter and CI coordinator. 
*   **Execution**:
    *   AST traversal, type inference, CFG compilation, and security rules logic execute entirely in Rust within the compiled `parser-v2` binary.
    *   No large syntactic AST structures are serialized to JSON.
    *   The binary outputs a lightweight, flat array of findings matching the `SecurityFinding` structure, keeping pipeline latency sub-second.

---

## 2. Capability Envelope

To guarantee correctness, the capability envelope of Parser v3 is explicitly defined:

### What Parser v3 WILL Support
*   **Single-Crate Type Inference**: Full lookup of custom structs, enums, and type aliases within the target program directory.
*   **Basic Control Flow splits**: Parsing of binary `if/else` statements, matching boolean variables, early returns (`return`, `?`, `require!`), and diverging panics (`panic!`, `assert!`).
*   **Alias propagation**: Tracking variable bindings to account fields and instruction args.
*   **SSA-lite state tracking**: Versioning local mutable variables to resolve re-assignments.

### What Parser v3 WILL NOT Support (Silent Safe Assumptions Prohibited)
*   **Workspace-Level Analysis**: Cross-crate references where types are defined in sibling workspace dependencies (resolves to `INCONCLUSIVE`).
*   **Trait Implementations & Dynamic Dispatch**: Evaluating calls resolved via generic trait bounds (resolves to `INCONCLUSIVE`).
*   **Recursive Loops (`loop`, `for`, `while`)**: Iterative mutations tracking loop invariants (resolves to `INCONCLUSIVE`).

### What Becomes `INCONCLUSIVE`
*   Functions containing workspace crate method calls, complex loop mutations, or dynamic dispatch.
*   Expressions that access types that cannot be resolved due to missing imports or compilation paths.

---

## 3. Module-Level Architecture

The refactored `parser-v2` crate is organized into the following Rust modules:

```
parser-v2/
├── Cargo.toml
└── src/
    ├── lib.rs            (Public interfaces & orchestrator)
    ├── workspace.rs      (File walker and AST visitor entry)
    ├── types.rs          (Global Struct/Enum registry definitions)
    ├── ast/
    │   ├── mod.rs        (Module definitions)
    │   ├── nodes.rs      (Function, Statement, and Expression structs)
    │   └── inference.rs  (New: Type Inference recursion engine)
    ├── cfg/
    │   ├── mod.rs        (CFG module declarations)
    │   ├── builder.rs    (New: syn statements -> CFG nodes compiler)
    │   └── ssa.rs        (New: Variable versioning tracker)
    └── security/
        ├── mod.rs        (Rule runner trait definitions)
        └── engine.rs     (New: Engine execution pipeline)
```

---

## 4. Dependency Graph

The execution flow of the compiler analysis pipeline is strictly linear:

```
[ syn::File Parser ]
         │
         ▼
[ AST Visitor (workspace.rs) ] ──► Registers Global Types to [ TypeRegistry ]
         │
         ▼
[ Type Inference (inference.rs) ] ◄── Queries [ TypeRegistry ]
         │
         ▼
[ SSA Versioner (ssa.rs) ]
         │
         ▼
[ CFG Compiler (builder.rs) ]
         │
         ▼
[ Security Engine (engine.rs) ] ──► Executes Security Rules
         │
         ▼
[ JSON Output (SecurityFinding) ]
```

---

## 5. Implementation Order (Sprints)

Optimized for a solo founder to build and test incrementally:

### Sprint 1: Type Inference & AST Nodes (`src/ast/`)
*   **Objective**: Implement `nodes.rs` and the `inference.rs` walker.
*   **Milestone**: Successfully print resolved types for nested expressions (e.g., matching `ctx.accounts.vault.owner` to `TypeRef::Pubkey`).
*   **Duration**: 3 Weeks

### Sprint 2: CFG Builder & Try Expander (`src/cfg/`)
*   **Objective**: Implement the CFG compiler (`builder.rs`) and explicit branch splits for `?` structures and `require!` macros.
*   **Milestone**: Compile correct graphs showing branch separations for conditionals.
*   **Duration**: 3 Weeks

### Sprint 3: SSA-lite Variable Versioner (`src/cfg/ssa.rs`)
*   **Objective**: Build the versioning table and resolve mutable assignments to verify alias versions are tracked correctly.
*   **Milestone**: Walk re-assigned bindings and verify the symbol table maps alias queries to the correct version index.
*   **Duration**: 2 Weeks

### Sprint 4: Security Engine Integration (`src/security/`)
*   **Objective**: Build the `SecurityRule` execution harness. Connect Zod parser schemas to load configurations.
*   **Milestone**: Run security pipeline and output findings JSON to CLI, terminating with exit codes.
*   **Duration**: 2 Weeks

---

## 6. Future-Proofing Analysis

The Parser v3 design provides a complete semantic model of program execution, ensuring that future implementation of core security rules will not require another parser rewrite.

### A. Missing Signer (`EPIC-SEC-002`)
*   *Why v3 supports it*: The CFG tracks branch guards and paths, while the `SymbolTable` traces aliases and type tags. Resolving if a signer check protects a privileged operation is simply a graph traversal verify check (checking if the path from the entry node to the instruction mutation contains a branch condition evaluating `signer.is_signer` or checks the type bounds of `Signer<'info>`).

### B. Owner Validation (`EPIC-SEC-001`)
*   *Why v3 supports it*: The `TypeInference` engine resolves whether an account is mapped to the safe `Account<'info, T>` wrapper. If it resolves to a raw `AccountInfo`, the CFG paths are evaluated to verify that a validation guard (e.g., `account.owner == program_id`) precedes the write statement.

### C. Reinitialization Protection (`EPIC-SEC-003`)
*   *Why v3 supports it*: The `Constraint AST Parser` extracts and processes Anchor initialization annotations (e.g., `init`, `init_if_needed`). The CFG evaluates whether writes to state variables are guarded by an initialization check.

### D. Close Authority Validation (`EPIC-SEC-004`)
*   *Why v3 supports it*: The CFG captures manual lamport mutation statements (`**lamports.borrow_mut() -= amount`), and the `SymbolTable` tracks structural types and destination accounts. The rules verify that the authority gating the lamport subtraction matches the program configuration.

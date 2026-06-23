# EPIC GuardFact Final Architecture Specification

This document defines the canonical Security Intermediate Representation (IR) specification for the EPIC Solana static analysis platform. It establishes the unified contract between framework-specific frontends (Producers) and analysis rules (Consumers).

---

## 1. Final Security IR Diagram

The architectural diagram below highlights data boundaries, ownership transitions, and where determinism is enforced.

```
       [SOURCE LAYER]                [COMPILER FRONTEND]             [SECURITY IR LAYER]            [RULES ENGINE]         [REPORTING]
       
  Rust Source / IDL Schema
             │
             ▼ (Frontend Extraction)
  ┌──────────────────────┐
  │  Framework Frontends │
  │ (Anchor/Shank/Codama)│
  └──────────┬───────────┘
             │
             │ (Data Ownership Transfer)
             ▼
  ┌────────────────────────────────────────────────────────┐
  │                   GuardFacts IR                        │
  │  - GuardTarget (SymbolId / SSAVersionId)               │
  │  - FactExpression (SolanaProperty)                     │  <- DETERMINISM ENFORCED HERE
  │  - FactConfidence (Declared / Asserted / Inconclusive)  │
  └──────────┬─────────────────────────────────────────────┘
             │
             │ (SSA & Type Resolution Binding)
             ▼
  ┌────────────────────────────────────────────────────────┐
  │               Analysis Engine Context                  │
  │  - CFG (Control Flow Graph)                            │
  │  - SSA-lite Version Map (Pre/Post DFS Interval)        │
  │  - Type Inference Engine (Type Walker)                 │
  └──────────┬─────────────────────────────────────────────┘
             │
             │ (Semantic Query Evaluation)
             ▼
  ┌────────────────────────────────────────────────────────┐
  │                  Security Rules Engine                 │
  │  - EPIC-SEC-001 (Owner Checks)                         │
  │  - EPIC-SEC-002 (Signer Checks)                        │
  │  - EPIC-SEC-003/004 (Init/Close Checks)                │
  └──────────┬─────────────────────────────────────────────┘
             │
             ▼ (Serialization)
  ┌────────────────────────────────────────────────────────┐
  │                  SARIF / CI Output                     │
  └────────────────────────────────────────────────────────┘
```

---

## 2. SSA Identity Model

To prevent false results caused by variable shadowing, aliasing, or reassignments, GuardFacts do not refer to variable names (Strings). They reference canonical SSA identities:

```rust
use serde::{Deserialize, Serialize};

/// A unique symbol key representing a logical variable/account in the context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SymbolId(pub usize);

/// Identifies a specific version of a symbol in the SSA-lite tracking system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SSAVersionId {
    pub symbol_id: SymbolId,
    pub version: usize,
}

/// The target of a security fact validation check.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GuardTarget {
    /// A specific version of a variable.
    Variable(SSAVersionId),
    /// A logical account parameter.
    Account(SymbolId),
    /// A static literal value.
    Literal(String),
}
```

### Concrete Evaluation Scopes

#### 1. Shadowing
```rust
let authority = ctx.accounts.admin; // Binding #1 -> SymbolId(1), version: 1 (authority#1)
let authority = ctx.accounts.owner; // Binding #2 -> SymbolId(2), version: 1 (authority#1 shadowed)
```
*   **Resolution**: The parser creates a new `SymbolId` for every `let` declaration. At statement 1, `authority` points to `SymbolId(1)`. At statement 2, `authority` resolves to `SymbolId(2)`. The facts remain distinct and bound to their respective unique Symbols, preventing shadowing conflicts.

#### 2. Aliasing
```rust
let signer = authority; // SymbolId(3) (signer#1) aliases SymbolId(1) (authority#1)
```
*   **Resolution**: The engine maps `SymbolId(3)` as an alias to `SymbolId(1)` using a Union-Find equivalence set. If a rule checks if `signer#1` is a signer, it looks up the alias group and resolves the fact bound to the root `SymbolId(1)`.

#### 3. Reassignment
```rust
authority = new_authority; // Reassignment -> SymbolId(1) version: 2 (authority#2)
```
*   **Resolution**: The SSA tracker increments the version of `SymbolId(1)` to `version: 2`. Any security fact verified on `authority#1` (version 1) does *not* carry over to `authority#2` (version 2) unless `new_authority` is also verified.

---

## 3. SVM-Level Invariants

All framework-specific concepts are removed from the Security IR. Below is the mapping and verification of variants:

| Original Fact Concept | Paradigm Type | SVM-Level Invariant Definition |
| :--- | :--- | :--- |
| **Signer** | (B) Solana Runtime | `GuardFact::Signer(GuardTarget)` |
| **Owner Check** | (B) Solana Runtime | `GuardFact::Owner { account: GuardTarget, expected_owner: FactExpression }` |
| **has_one (Anchor)** | (A) Anchor Concept | Mapped to `GuardFact::KeyRelation { account, field: SolanaProperty::Address, target }` |
| **close (Anchor)** | (A) Anchor Concept | Mapped to `GuardFact::Deallocated { account, destination }` |
| **realloc (Anchor)** | (A) Anchor Concept | Mapped to `GuardFact::Resized { account, new_size, payer }` |
| **init (Anchor)** | (A) Anchor Concept | Mapped to `GuardFact::Initialized { account, payer, space }` |

Only runtime invariants belong in the canonical IR:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuardFact {
    Signer(GuardTarget),
    Owner {
        account: GuardTarget,
        expected_owner: FactExpression,
    },
    KeyRelation {
        account: GuardTarget,
        field: SolanaProperty,
        target: GuardTarget,
    },
    PDA {
        account: GuardTarget,
        seeds: Vec<FactExpression>,
        bump: Option<FactExpression>,
    },
    Initialized {
        account: GuardTarget,
        payer: GuardTarget,
        space: Option<FactExpression>,
    },
    Resized {
        account: GuardTarget,
        new_size: FactExpression,
        payer: GuardTarget,
    },
    Deallocated {
        account: GuardTarget,
        destination: GuardTarget,
    },
    Custom {
        namespace: String,
        kind: String,
        payload: Vec<FactExpression>,
    },
}
```

---

## 4. Open Extensibility & Guardrails

To allow custom plugins (Token-2022, timelocks, multisig) while maintaining determinism, the `GuardFact::Custom` variant implements the following design constraints:

1.  **Ownership Model**: Custom facts are strictly owned by `InstructionAnalysisContext` and must use the serializable `FactExpression` AST. No external custom runtime logic or callbacks are allowed during rule evaluation.
2.  **Namespace Model**: Custom facts must use reverse-DNS namespaces (e.g. `spl.token2022.extension`) to prevent identifier collisions.
3.  **Validation Model**: Custom facts can only bind to `GuardTarget` variants. They cannot reference unmapped memory or raw untracked registers.
4.  **Compatibility Guarantees**: A custom fact is ignored by default by the rules engine. Rules must explicitly subscribe to a custom fact namespace to query it, ensuring core execution invariants are never altered.

---

## 5. Dominance Representation Evaluation

We evaluated three architectures for representing control-flow dominance:

| Architecture Metric | Dominator Tree IDs | Dominance Regions | DFS Interval Indexing (Selected) |
| :--- | :--- | :--- | :--- |
| **Memory Complexity** | $O(V)$ | $O(V \times R)$ | **$O(V)$** (Fixed 2 integers per node) |
| **Query Complexity** | $O(\text{Depth})$ tree traversal | $O(1)$ set lookup | **$O(1)$** (Two integer comparisons) |
| **Maintenance Cost** | Medium | High | **Low** (Calculated in a single DFS pass) |
| **Compatibility** | High | Low | **High** (Standard compiler method) |

### Selected Design: DFS Interval Indexing
*   **Justification**: DFS interval indexing computes `dfs_entry` and `dfs_exit` values for each node in the Dominator Tree. Node `A` dominates node `B` if and only if:
    $$\text{dfs\_entry}(A) \le \text{dfs\_entry}(B) \quad \text{AND} \quad \text{dfs\_exit}(A) \ge \text{dfs\_exit}(B)$$
    This approach is highly scalable and executes checks in constant ($O(1)$) time.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardFactLocation {
    pub node_id: usize,
    pub statement_index: Option<usize>,
    pub dominance_interval: DominanceInterval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DominanceInterval {
    pub dfs_entry: usize,
    pub dfs_exit: usize,
}
```

---

## 6. Framework Neutrality & FactExpression Audit

To prevent Rust-specific assumptions (e.g., method calls like `.key()`) from leaking into the engine, `FactExpression` is normalized to Solana Virtual Machine physical properties:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactExpression {
    Target(GuardTarget),
    Literal(String),
    PropertyOf {
        target: GuardTarget,
        property: SolanaProperty,
    },
    BinaryOp {
        op: String,
        lhs: Box<FactExpression>,
        rhs: Box<FactExpression>,
    },
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SolanaProperty {
    Address,
    Owner,
    Lamports,
    DataLength,
    IsSigner,
    IsWritable,
    Executable,
}
```

This ensures that metadata-derived facts (from Codama/Shank JSON schema files) are modeled identically to code-derived facts (from Anchor/Native Rust source code).

---

## 7. Fact Confidence Model

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactConfidence {
    /// Enforced structurally via context type declarations.
    Declared,
    /// Asserted procedurally via control-flow checks.
    Asserted,
    /// Check cannot be statically verified (fails closed).
    Inconclusive,
}
```

---

## 8. Migration, Compatibility & Future Framework Analysis

### Migration Impact
*   **Parser v3**: Replaces synthetic node generation with simple attribute conversion mapping to `GuardFacts`.
*   **CFG & SSA-lite**: CFG represents actual code statements, keeping line numbers and SSA version tracking clean.

### Backward Compatibility
*   `ControlFlowGraph` uses `#[serde(default)]` on `ssa_states` and `guard_facts` to ensure older version schemas remain compatible.

### Future Framework Analysis
*   **Anchor**: Translator maps declarative struct type definitions directly to `GuardFact::Owner` and `GuardFact::Signer` (Declared).
*   **Pinocchio / Native**: Traverse procedural assertions (e.g. `assert_keys_eq!`) and promote them to `GuardFact::KeyRelation` (Asserted).
*   **Shank / Codama**: IDL parsers emit SVM-level `GuardFacts` without parsing Rust source code.

---

## 9. Final Sign-Off Recommendation

1.  **Is GuardFact a valid long-term Security IR?**
    Yes. Normalizing constraints to SVM-level invariants and using SSA-aware identifiers ensures the IR remains correct, deterministic, and future-proof.
2.  **Would you approve implementation today?**
    Yes. All critical issues from the architectural review (variable identities, dominance indexing, and Anchor leakage) have been resolved.
3.  **What architectural changes are still mandatory?**
    None.
4.  **What changes are optional but recommended?**
    Implement a utility class in the parser module to convert `FactExpression` structures to readable terminal text for easier debugging.
5.  **What technical debt remains?**
    Mapping complex custom constraints in Anchor (e.g. `#[account(constraint = a.calc() > 0)]`) will produce `FactExpression::Unknown` facts (failing closed to `Inconclusive`) until the type walker supports full method invocation body expansion. This does not impact safety.

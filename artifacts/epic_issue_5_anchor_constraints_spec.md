# Architectural Specification: Anchor Constraint & Attribute Analysis Engine (Issue #5)

## 1. Executive Summary & Core Paradigm
In the Anchor framework, account constraints (declared via `#[account(...)]` macro attributes on structs implementing `derive(Accounts)`) are compiled into runtime validation code that executes *prior* to the instruction handler body. Treating these attributes simply as parsed comment metadata creates a disconnect in control-flow analysis: security analysis engines are forced to maintain two separate validation state representations (the AST/CFG and the out-of-band attribute lists) and guess how they relate.

**The Core Paradigm of this Design:**
We resolve this by translating Anchor constraints directly into **First-Class Security Guard Nodes** within the Control Flow Graph (CFG). Macro attributes are parsed into a structured AST representation, which the CFG Builder then compiles into sequential assertion-equivalent nodes prepended directly to the instruction handler body. 

By unifying declarative constraints with explicit procedural checks into the same SSA-lite CFG model, downstream analysis engines (e.g. signer verification, owner check, reinitialization rules) can query them identically.

---

## 2. Anchor Constraint Grammar & AST Representation
We model Anchor constraints as explicit AST structures rather than unparsed raw tokens.

### Rust Data Structures (`ast/constraints.rs`)
```rust
use serde::{Deserialize, Serialize};
use crate::ast::ExpressionNode;

/// Represents a single parsed field inside an Anchor `derive(Accounts)` struct.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnchorAccountField {
    pub name: String,
    pub account_type: String, // e.g., "Account", "AccountLoader", "Signer", "Program"
    pub inner_type: Option<String>, // e.g., "VaultState" inside "Account<'info, VaultState>"
    pub constraints: Vec<AnchorConstraint>,
}

/// Enumerate all supported Anchor declarative constraints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnchorConstraint {
    /// #[account(mut)]
    Mut,
    /// #[account(signer)]
    Signer,
    /// #[account(owner = <expr>)]
    Owner(ExpressionNode),
    /// #[account(has_one = <target>)]
    HasOne {
        field_name: String,
        target: ExpressionNode,
    },
    /// #[account(constraint = <expr>)]
    Constraint(ExpressionNode),
    /// #[account(close = <target>)]
    Close(ExpressionNode),
    /// #[account(seeds = [<expr>, ...], bump = <expr>)]
    Seeds {
        seeds: Vec<ExpressionNode>,
        bump: Option<ExpressionNode>,
    },
    /// #[account(init, payer = <expr>, space = <expr>)]
    Init {
        payer: ExpressionNode,
        space: Option<ExpressionNode>,
    },
    /// #[account(init_if_needed, payer = <expr>, space = <expr>)]
    InitIfNeeded {
        payer: ExpressionNode,
        space: Option<ExpressionNode>,
    },
    /// #[account(realloc = <expr>, payer = <expr>, zero = <bool>)]
    Realloc {
        space: ExpressionNode,
        payer: ExpressionNode,
        zero: bool,
    },
}
```

---

## 3. CFG Integration & First-Class Security Guards

### Prepending Security Guard Nodes
Before entering Node 0 (the entry point of the instruction handler body), the CFG Builder injects a sequence of **Guard Nodes** (Nodes -1, -2, etc.). Each guard node corresponds to a specific Anchor validation check.

```
                    [Macro Attribute Constraints]
                                  │
                                  ▼
                   ┌──────────────────────────────┐
                   │  Node -2: Guard (mut check)  │  <- e.g. assert!(vault.is_writable)
                   └──────────────┬───────────────┘
                                  │
                                  ▼
                ┌───────────────────────────────────┐
                │ Node -1: Guard (owner validation) │  <- e.g. assert!(vault.owner == program_id)
                └──────────────┬────────────────────┘
                                  │
                                  ▼
                     ┌─────────────────────────┐
                     │ Node 0: Instruction Body│  <- Start of user code
                     └─────────────────────────┘
```

Each declarative constraint translates into a structured `StatementNode` inside these guard nodes:
*   `#[account(mut)]` $\rightarrow$ `assert!(account.is_writable == true)`
*   `#[account(signer)]` $\rightarrow$ `assert!(account.is_signer == true)`
*   `#[account(has_one = authority)]` $\rightarrow$ `assert!(account.authority == authority.key())`
*   `#[account(constraint = vault.amount > 0)]` $\rightarrow$ `assert!(vault.amount > 0)`
*   `#[account(owner = custom_program)]` $\rightarrow$ `assert!(account.owner == custom_program)`

### Integration with SSA-lite Versioning
Because these Guard Nodes are prepended to Node 0, the SSA-lite variable tracking pass automatically processes them first.
1. Parameters and instruction fields (e.g. `vault#1`, `authority#1`) are declared at Node -N.
2. The assertion statements register variable accesses, binding types and versions.
3. Downstream security rules can perform queries like:
   *"Does there exist a path from entry to critical instruction statement S where the active version of `vault` was not subject to an owner validation guard?"*

---

## 4. Scope and Boundary Definition

### Supported in v1 (Deterministic Parsing)
*   **Signer & Mut Constraints**: Raw presence mapping to boolean checks.
*   **HasOne & Basic Constraints**: Literal expression mapping (e.g. `has_one = authority`).
*   **Seeds & Bumps**: Deterministic extraction of seed arrays and bumps for derivation validation.
*   **Init / Realloc Expressions**: Extraction of payer and space definitions.

### INCONCLUSIVE Boundary Conditions (Fail-Closed)
To eliminate assumptions, the engine marks the analysis as `INCONCLUSIVE` under the following conditions:
*   **External/Cross-Struct Constraints**: Constraints referencing structs or types outside of the current instruction's `derive(Accounts)` context that cannot be resolved in the `TypeRegistry`.
*   **Ambiguous Seed Expressions**: Seed arrays containing runtime function calls or calculations whose types are unresolved (e.g. `seeds = [vault.key().as_ref(), &custom_helper(a)]`).
*   **Dynamic CPI-bound validation**: Dynamic constraints validated via external CPI calls instead of standard checks.

---

## 5. Threat Model & Analysis Capabilities

The model directly enables robust detection of major Solana security issues:

| Threat / Vulnerability | Anchor Attribute / Constraint Guard | Analysis Query Detection Strategy |
| :--- | :--- | :--- |
| **Missing Signer Check** (Sec-002) | `#[account(signer)]` | Verify that the signer guard node exists for the target account and dominates any critical mutable operation in the CFG. |
| **Missing Owner Check** (Sec-001) | `#[account(owner = ...)]`, implicit `Account<'info, T>` | Ensure a Program Owner Check guard is active. If the type is `Account<'info, T>`, verify the implicit discriminator owner validation guard is prepended. |
| **Reinitialization / Reprovide** | `#[account(init)]`, `#[account(init_if_needed)]` | Locate the initialization guard node. Ensure that it enforces account serialization boundaries and does not allow execution flow without zeroing or state resets. |
| **Arbitrary Realloc Overruns** | `#[account(realloc = ...)]` | Validate that the size parameter expression uses deterministic type sizing (`8 + size_of::<T>()`) and the payer is a validated signer. |

---

## 6. Hostile Design Review: "Why This Design Could Fail"

To ensure architectural integrity, we must evaluate potential points of failure:

### 1. The Context Propagation Gap (Multi-version Attribute Shifts)
*   **Failure Scenario**: Different versions of the Anchor framework parse and compile constraints differently (e.g. Anchor 0.28 vs Anchor 0.30 introduces structural changes to seeds or default layout validation). If the parser does not know the specific target Anchor version, translating constraints into hardcoded assert-equivalents can introduce false-positive validation errors or false-negatives due to missed implicit validations.
*   **Mitigation**: The engine must inspect the Anchor version in Cargo.toml and parameterized the translation logic based on target Anchor edition version specs.

### 2. Implicit Checks Erasure
*   **Failure Scenario**: Anchor executes many implicit checks. For instance, using the type `Account<'info, VaultState>` implicitly checks that the account owner is the current program ID, whereas `UncheckedAccount<'info>` does not. If the parser only models explicit constraints like `#[account(...)]` and fails to generate the implicit type-based guard nodes, the analysis will suffer false-negatives (missing validations that are actually present implicitly).
*   **Mitigation**: The translation engine must look up the account field types in the `TypeRegistry` and inject corresponding implicit guard nodes (e.g., owner check for `Account`, deserialization discriminator checks) along with explicit constraints.

### 3. Evaluation Failure of Arbitrary Expressions
*   **Failure Scenario**: A constraint uses complex rust expressions, e.g. `#[account(constraint = a.calc(b) == c.val())]`. If our AST or type inference engine cannot evaluate `calc(b)` (due to out-of-scope method body), it becomes `ExpressionKind::Unresolved`. If we just ignore it, we miss a security guard. If we fail closed, we mark the whole analysis `INCONCLUSIVE`. A high rate of `INCONCLUSIVE` results renders the engine useless.
*   **Mitigation**: Define strict heuristic fallbacks that parse the method call as a black-box identifier transition, rather than immediately marking the whole scope `INCONCLUSIVE`, while strictly preserving the variable version bindings.

---

## 7. Historical Validation Plan
To validate the engine before writing rules, we will execute a historical regression suite:
1.  **Fixture Collection**: Extract `derive(Accounts)` code from 20+ open-source Anchor projects spanning multiple Anchor versions.
2.  **Constraint Parity Verification**: Compile the target programs, locate the Anchor-generated validation code in rustc intermediate representation (cargo expand), and assert structural equivalence between our generated CFG Guard Nodes and Anchor's generated validation checks.
3.  **Strict Correctness Bound**: 100% of standard declarative constraints must map to guards without manual intervention. Any dynamic method-based constraint must fail closed safely.

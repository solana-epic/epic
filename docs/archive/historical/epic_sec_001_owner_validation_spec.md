# EPIC-SEC-001: Owner Validation Architectural Specification

This document details the canonical architectural specification for the **EPIC-SEC-001 Owner Validation** static analysis rule in the EPIC Solana static analysis platform. It defines the threat model, detection algorithm, GuardFact and CFG/SSA-lite integration, path-sensitive symbol resolver design, safety classification boundaries, historical validation plan, and a hostile security self-review.

---

## 1. Rule Definition & Threat Model

### Rule Definition
* **Rule ID**: `EPIC-SEC-001`
* **Name**: Owner Validation Rule
* **Severity**: `Critical`
* **Target Vulnerability**: Missing, incorrect, or bypassed owner checks on user-supplied accounts.
* **Core Invariant**: Every execution path in a Solana instruction handler that performs a mutable state write (or reads state that influences critical control-flow paths) on an account parameter must be dominated by a program ownership validation check that restricts the account's owner to a trusted program ID and fails-closed on mismatch.

### Threat Model: The Solana Owner Validation Vulnerability
Unlike EVM programs, which run in an address space managed directly by the ledger state where contract code is bound to the address, Solana decouples data storage from execution logic. In the Solana Virtual Machine (SVM) programming model:
1. **Untrusted Accounts Payload**: An instruction caller can supply any arbitrary list of account addresses to an instruction handler.
2. **Account Deserialization**: The instruction handler deserializes the account data bytes into structured memory representations (e.g., struct fields, state states).
3. **Lack of Native Isolation**: By default, the SVM runtime only enforces that a program cannot write to account data unless that program is the owner of the account. However, it does *not* prevent a program from deserializing and reading data from an account owned by an arbitrary program, nor does it prevent the program from executing logic based on that fake data, or writing to accounts that *are* owned by the program but whose state was initialized or modified by a fake account reference.

```
Attacker passes:
[Fake Account (Address X)] ──► [Deserialization in Program] ──► [Read Spoofed Data]
                                                                        │
                                                                        ▼
                                                              [State/Balance Spoofed]
                                                                        │
                                                                        ▼
                                                            [Unauthorized state writes
                                                             on legitimate accounts]
```

If a program deserializes an account structure (e.g. `UserAccountInfo`) without verifying that `account.owner == program_id` (or a trusted system/token program owner), an attacker can create a **fake account** with identical data layouts containing spoofed state parameters (e.g., setting `attacker.balance = 1_000_000` or `attacker.is_admin = true`). Without owner validation, the program reads the fake account, trusts the deserialized layout, and proceeds to perform mutable state changes or token transfers based on fake data.

---

## 2. SVM-Level Operational Boundaries (Safety Classification)

To ensure deterministic, fail-closed static verification, the rule engine classifies account references at each instruction statement into three categories:

| Safety Category | Definition | Concrete Criteria |
| :--- | :--- | :--- |
| **SAFE** | The account write is strictly dominated by a valid ownership check. | 1. The account is validated by a structural wrapper (`Account`, `AccountLoader`, `InterfaceAccount`) that generates a `GuardFact::Owner` at instruction entry.<br>2. An explicit procedural check (e.g., `*account.owner == expected_owner`) dominates the write, and the failure branch terminates execution.<br>3. A CPI call to a program that implicitly enforces ownership (like a token program `transfer` CPI on a token account) dominates the write. |
| **UNSAFE** | A mutable write is performed without dominating owner validation. | 1. A write occurs on an unchecked account (`UncheckedAccount`, `AccountInfo`) without any dominating `GuardFact::Owner`.<br>2. An owner check is present in the CFG but exists on a parallel conditional branch or occurs *after* the write.<br>3. An owner check is present, but the failure path does not abort execution (e.g., logs a message and continues).<br>4. The owner check compares the account owner to a user-controlled parameter. |
| **INCONCLUSIVE** | Static analysis cannot verify the owner check status. | 1. The account type is unresolved in the [TypeRegistry](file:///Users/aksh/Documents/Solana%20EPIC/packages/parser-v2/src/cfg/ssa.rs#L9).<br>2. The owner verification logic uses complex custom calculations or dynamic external helper functions that are opaque to static analysis.<br>3. Pointer arithmetic or dynamic offset indexing obscures the identity of the account being written to.<br><br>**Fails-Closed Policy**: In strict mode, `INCONCLUSIVE` is treated as `UNSAFE` (emits a critical finding). In audit mode, it is reported as a high-severity warning requiring manual review. |

---

## 3. Account Wrapper Support Matrix

EPIC-SEC-001 explicitly supports all standard Solana Rust and Anchor account wrappers. The extraction engine maps these wrappers to SVM-level invariants:

### 1. `Account<'info, T>`
* **Framework Behavior**: Anchor automatically verifies that the account's owner is the program ID of the program implementing type `T` (or the program ID specified in attributes like `#[account(owner = owner_expr)]`).
* **GuardFact Mapping**: Emits `GuardFact::Owner { account: GuardTarget::Account(sym_id), expected_owner: FactExpression::Literal("program_id") }` (or the converted `owner_expr` expression) with `FactConfidence::Declared` at the entry block (`node_id: 0`).
* **Rule Assessment**: **SAFE**. Dominates all writes because it is declared at the entry node of the CFG.

### 2. `AccountLoader<'info, T>`
* **Framework Behavior**: Used for zero-copy deserialization of large accounts. Anchor structurally validates program ownership upon account loading.
* **GuardFact Mapping**: Emits `GuardFact::Owner { account: GuardTarget::Account(sym_id), expected_owner: FactExpression::Literal("program_id") }` with `FactConfidence::Declared` at `node_id: 0`.
* **Rule Assessment**: **SAFE**.

### 3. `InterfaceAccount<'info, T>`
* **Framework Behavior**: Validates that the account owner matches one of the program IDs implementing interface `T`.
* **GuardFact Mapping**: Emits:
  ```rust
  GuardFact::Owner {
      account: GuardTarget::Account(sym_id),
      expected_owner: FactExpression::BinaryOp {
          op: "||".to_string(),
          lhs: Box::new(FactExpression::Literal("program_id_a".to_string())),
          rhs: Box::new(FactExpression::Literal("program_id_b".to_string())),
      }
  }
  ```
  with `FactConfidence::Declared`.
* **Rule Assessment**: **SAFE** if the program IDs in the disjunction are trusted by the rule engine.

### 4. `UncheckedAccount<'info>` / `AccountInfo<'info>`
* **Framework Behavior**: Raw account wrappers. No implicit ownership check is performed.
* **GuardFact Mapping**: Emits no implicit facts.
* **Rule Assessment**: **UNSAFE** unless a procedural assertion check is identified in the CFG and promoted to a `GuardFact::Owner { ..., confidence: Asserted }` which dominates the write path.

---

## 4. Path-Sensitive Symbol Resolver Design

To bridge the gap between AST syntax and static security facts, EPIC employs a path-sensitive symbol resolver. The resolver tracks active scope version mappings and resolves variable names, aliases, and nested field paths back to their canonical `SymbolId` and active `SSAVersionId`.

### Data Structures
```rust
use std::collections::HashMap;
use crate::cfg::guards::{SymbolId, SSAVersionId};

/// Tracks the active symbol resolution and alias mappings at a specific statement.
#[derive(Debug, Clone, Default)]
pub struct SymbolResolver {
    /// Maps a versioned SSA variable string representation (e.g., "vault#2") to its canonical SymbolId.
    pub alias_map: HashMap<String, SymbolId>,
    /// Maps nested path string representations (e.g., "ctx.accounts.vault") to their root SymbolId.
    pub path_map: HashMap<String, SymbolId>,
    /// Maps SymbolIds to their canonical alias equivalents (equivalence classes).
    pub equivalence_map: HashMap<SymbolId, SymbolId>,
}
```

### Scope Resolution Scenarios

#### Scenario 1: Shadowing
Shadowed variables must not bleed security facts between distinct instances.
```rust
// Scope 1
let authority = ctx.accounts.admin; // authority#1 -> SymbolId(1) [Admin account, checked]
// Scope 2 (Nested Block)
{
    let authority = ctx.accounts.owner; // authority#2 -> SymbolId(2) [Owner account, unchecked]
    // Writes to authority resolve to SymbolId(2) (authority#2)
}
// Scope 1 resumes
// Writes to authority resolve back to SymbolId(1) (authority#1)
```
* **Mechanism**: During CFG traversal, when a nested scope is entered, the resolver pushes a lexical scope layer. On lookup, it queries variables in reverse order of scope depth. Resolving `authority` inside the block returns `authority#2` -> `SymbolId(2)`. When exiting the block, `authority#2` is popped, restoring the active binding `authority#1` -> `SymbolId(1)`.

#### Scenario 2: Aliasing
Aliasing binds a new variable version to an existing account's fact context.
```rust
let vault_alias = ctx.accounts.vault; // vault_alias#1 aliases ctx.accounts.vault (SymbolId(3))
```
* **Mechanism**: The assignment pattern is analyzed. The initializer `ctx.accounts.vault` resolves to `SymbolId(3)`. The resolver maps `alias_map.insert("vault_alias#1", SymbolId(3))`. Any query for `vault_alias#1` resolves directly to `SymbolId(3)`, inheriting all ownership facts validated on `vault`.

#### Scenario 3: Reassignment
Reassignments increment the variable version and break the association with the previous symbol's facts.
```rust
let mut active_vault = ctx.accounts.vault_a; // active_vault#1 -> SymbolId(4) [Checked]
active_vault = ctx.accounts.vault_b;        // active_vault#2 -> SymbolId(5) [Unchecked]
// Write to active_vault occurs here.
```
* **Mechanism**: SSA version counters track that at the write statement, the active version is `active_vault#2`. The resolver maps `active_vault#2` to `SymbolId(5)`. Since `SymbolId(5)` has no owner check, the write is flagged as **UNSAFE**, preventing the check on `vault_a` from falsely certifying the write on `vault_b`.

#### Scenario 4: Nested Path Fields
Resolving path patterns like `ctx.accounts.vault.lamports.borrow_mut()` to the root symbol.
* **Mechanism**: The path parser walks the AST fields. When encountering `ctx.accounts.X`, it ignores subfields like `.lamports`, `.data`, `.owner` or method dereferencing, and extracts the root field name `X`. It queries `path_map` to retrieve the `SymbolId` for parameter `X`.

---

## 5. Control Flow Graph & Dominance Validation Algorithm

The dominance check ensures that no execution path can reach a write statement without passing through a validation node. We use **DFS Interval Indexing** on the Dominator Tree for constant-time ($O(1)$) dominance verification.

```
       [CFG Node 0: Entry]
       (Extracts GuardFacts)
                │
                ▼
      [CFG Node 1: Owner Check] ──► Failure branch: [CFG Node 2: Abort/Error]
                │
                ▼
       [CFG Node 3: Processing]
                │
                ▼
       [CFG Node 4: Write Site]  <── (Dominance check verifies Node 1 dominates Node 4)
```

### Verification Algorithm Steps

#### Step 1: Pre-compute Dominator DFS Intervals
1. Construct the Dominator Tree of the CFG using the Lengauer-Tarjan algorithm.
2. Perform a Depth-First Search (DFS) traversal on the Dominator Tree starting from the entry node.
3. Maintain a counter to assign `dfs_entry` and `dfs_exit` indices for each node:
   ```rust
   fn dfs_index(node, counter, intervals) {
       intervals[node].dfs_entry = counter;
       counter += 1;
       for child in dominator_tree.children(node) {
           dfs_index(child, counter, intervals);
       }
       intervals[node].dfs_exit = counter;
       counter += 1;
   }
   ```
4. Dominance condition check: Node `A` dominates Node `B` if and only if:
   $$\text{dfs\_entry}(A) \le \text{dfs\_entry}(B) \quad \text{AND} \quad \text{dfs\_exit}(A) \ge \text{dfs\_exit}(B)$$

#### Step 2: Identify Write Sites
Traverse all nodes and statements in the CFG to find mutable account writes:
* Assignments to fields: `account.x = y`
* Method calls: `.borrow_mut()`, `.try_borrow_mut()`, `.try_borrow_mut_data()`
* Extraction of mutable references: `&mut account.data`

For each write site `w`, determine:
* `Node_w`: The CFG node containing the write.
* `StmtIdx_w`: The statement index within the node.
* `SymbolId_w`: The canonical root symbol of the account resolved via `SymbolResolver`.

#### Step 3: Evaluate Ownership Dominance
For each identified write site `w` on `SymbolId_w`:
1. Find all `GuardFact::Owner { account, expected_owner }` facts where `account` resolves to `SymbolId_w`.
2. For each matching fact `f` located at node `Node_f` and statement index `StmtIdx_f`:
   * Check if `Node_f` dominates `Node_w` using the DFS interval check.
   * If `Node_f == Node_w`, verify statement order: `StmtIdx_f < StmtIdx_w`.
3. If no dominating owner fact is found, classify the path as **UNSAFE** and emit a finding.

#### Step 4: Verify Fail-Closed Path Exit (for Asserted Facts)
If the dominating fact `f` has confidence `FactConfidence::Asserted` (a procedural check in code):
1. Locate the conditional branching node `Node_check` corresponding to the owner check (e.g., `if account.owner != expected_owner`).
2. Identify the failure branch edge (the path taken when the owner check fails).
3. Verify that the failure branch path terminates execution:
   * Perform a reachability traversal starting from the failure branch target node.
   * Assert that **every** reachable terminal node in this sub-graph is an abort/exit node (e.g., panics, returns an error).
   * Verify that the write node `Node_w` is **not** reachable from the failure branch path.
4. If the failure path can merge back into the main execution flow or reach `Node_w`, classify as **UNSAFE**.

---

## 6. Historical Validation Cases

We validate EPIC-SEC-001 against real-world Solana program vulnerability profiles.

### Case A: The Cashio Exploit (Infinite Mint)

#### Exploit Profile
In the Cashio print instruction, the program deserialized a `collateral_metadata` account and read its decimals and collateral parameters to calculate how many cash tokens to mint. However, the account was passed as raw `AccountInfo` (effectively `UncheckedAccount`), and the program did not perform any owner validation to verify it was owned by the expected program. The attacker passed a fake metadata account containing custom decimals, causing the program to print billions of tokens.

#### Vulnerable Code Pattern (Native/Anchor Mix)
```rust
// Accounts Struct
#[derive(Accounts)]
pub struct PrintCash<'info> {
    pub collateral_metadata: AccountInfo<'info>, // UncheckedAccount
    pub cash_mint: Account<'info, Mint>,         // Implicitly Checked
    // ...
}

pub fn print_cash(ctx: Context<PrintCash>, amount: u64) -> Result<()> {
    // DESERIALIZATION & READ WITHOUT OWNER VALIDATION
    let metadata = CollateralMetadata::try_from_slice(*ctx.accounts.collateral_metadata.data.borrow())?;
    
    // Write occurs on another state (Minting tokens)
    token::mint_to(
        ctx.accounts.into_mint_context(),
        metadata.calculation_constant * amount,
    )?;
    Ok(())
}
```

#### EPIC-SEC-001 Walkthrough
1. **Symbol Table**:
   * `collateral_metadata` -> `SymbolId(0)`
   * `cash_mint` -> `SymbolId(1)`
2. **Fact Ingestion**:
   * `cash_mint` emits a `GuardFact::Owner { account: GuardTarget::Account(SymbolId(1)), expected_owner: TokenProgram }` (Declared, `node_id: 0`).
   * `collateral_metadata` emits no owner facts.
3. **CFG Traversal**:
   * Identifies a state read on `collateral_metadata` that directly controls/influences the parameters of the mutable write (mint_to CPI).
   * Identifies the read site: `let metadata = CollateralMetadata::try_from_slice(*ctx.accounts.collateral_metadata.data.borrow())?`.
4. **Dominance Evaluation**:
   * Target is `SymbolId(0)`.
   * Search `guard_facts` list for `GuardFact::Owner` for `SymbolId(0)`.
   * **Result**: Empty list.
5. **Verdict**: Flagged as **UNSAFE**. Detection successful.

---

### Case B: The Crema Finance Exploit (Fake Tick Array)

#### Exploit Profile
The Crema Finance program allowed users to perform swaps. The swap calculation depended on a series of `tick_array` accounts. The program deserialized the user-supplied tick array accounts but failed to verify that the owner of these tick arrays was the Crema Concentrated Liquidity program. The attacker passed a fake tick array account initialized with spoofed tick data, manipulating price rates during the swap to drain pools.

#### Vulnerable Code Pattern
```rust
pub fn swap(ctx: Context<Swap>, path: Vec<Pubkey>) -> Result<()> {
    let tick_array_info = &ctx.remaining_accounts[0]; // Resolves to AccountInfo
    
    // Reads tick array data without checking owner
    let tick_array = TickArray::deserialize(&mut &**tick_array_info.data.borrow())?;
    
    // Perform state updates and write values
    let pool = &mut ctx.accounts.pool;
    pool.update_state_with_ticks(&tick_array)?;
    Ok(())
}
```

#### EPIC-SEC-001 Walkthrough
1. **Symbol Table**:
   * `remaining_accounts[0]` -> `SymbolId(3)`
   * `pool` -> `SymbolId(4)`
2. **Fact Ingestion**:
   * `pool` has `GuardFact::Owner { account: SymbolId(4), expected_owner: program_id }` (Declared).
   * `remaining_accounts[0]` has no owner facts.
3. **CFG Traversal**:
   * Detects read of `remaining_accounts[0]` (`SymbolId(3)`) which flows into state update parameter calculation of `pool` (`SymbolId(4)`).
4. **Dominance Check**:
   * Looks up `GuardFact::Owner` for `SymbolId(3)`.
   * **Result**: None.
5. **Verdict**: Flagged as **UNSAFE** (Critical vulnerability path detected).

---

## 7. Hostile Self-Review & Defense-in-Depth Analysis

### Attack Vector 1: Indirect/Parameter-Controlled Owner Validation
An attacker attempts to satisfy the static analysis engine by adding an owner comparison, but compares the owner against an unvalidated or attacker-supplied parameter.
```rust
// Attacker supplies a fake expected_owner account.
if account.owner != ctx.accounts.attacker_provided_program.key() {
    return Err(ErrorCode::InvalidOwner.into());
}
```
* **Bypass Potential**: If the rule engine only verifies the *existence* of a `GuardFact::Owner` without evaluating the `expected_owner` expression, it will classify the account as SAFE.
* **EPIC Mitigation**: The dominance solver evaluates the `expected_owner` expression in the `GuardFact`. It must match:
  1. A constant program ID literal.
  2. The executing program's address (`program_id`).
  3. A known trusted system program ID from the static configuration database.
  If `expected_owner` is a dynamic variable (e.g., `attacker_provided_program`), the engine marks it as `FactConfidence::Inconclusive` and raises an UNSAFE alert.

### Attack Vector 2: Hashed/Transitive Owner Validation
A developer checks program ownership using a custom hashing or key check function, or validates the owner transitively through an intermediary state.
```rust
let is_valid = check_custom_hash(account.owner.to_bytes());
require!(is_valid, Error::InvalidOwner);
```
* **Bypass Potential**: The static analysis engine cannot parse custom cryptography or helper functions, leading to false positives on secure code.
* **EPIC Mitigation**: The rule engine implements the **Fails-Closed** design. Because `check_custom_hash` cannot be resolved to a standard `GuardFact::Owner` invariant, the fact defaults to `Inconclusive`. In standard audit mode, EPIC flags this for manual review, preventing silent bypasses.

### False-Positive Scenario: System Program Initialization (CPI)
During account initialization, a program creates an account via the System Program CPI. At this stage, the account is owned by the System Program. The program immediately performs state writes to configure the account *before* executing the transfer of ownership to itself.
```rust
// 1. Create account via System Program (Owner = System Program)
system_program::create_account(ctx.accounts.into_create_context(), ...)?;
// 2. Perform initial state write (Owner is STILL System Program)
let mut data = account.try_borrow_mut_data()?;
data[0] = 1; // Write Site
// 3. Assign ownership to self
state::assign_ownership(account, program_id)?;
```
* **Why it triggers**: Static analysis sees a write on `account` (at step 2) but the owner check (or assignment check to the program ID) only occurs at step 3. Since the owner validation does not dominate the write, it flags a false positive.
* **Mitigation**: The extraction engine recognizes the `GuardFact::Initialized` fact. If the CFG contains a dominating `GuardFact::Initialized { account, payer, .. }` fact matching the write target, writes inside the initialization window are exempted from the program-owner check requirements.

### False-Negative Scenario: Partial Program-Owner Checks
A developer performs owner checks on some but not all elements of an account array.
```rust
for acc in &ctx.remaining_accounts {
    // If the loop exits early or can skip items
    if acc.key() == target_key {
        require_keys_eq!(*acc.owner, program_id);
    }
    // Write occurs on all accounts in remaining_accounts
    acc.try_borrow_mut_data()?;
}
```
* **Why it triggers**: The check only dominates writes when `acc.key() == target_key`. Other accounts bypass the check.
* **Mitigation**: Path-sensitive traversal checks the dominance of each element write. In loop constructs, if the loop condition or body allows paths where the check is bypassed, the dominance check over the write site fails.

---

## 8. Implementation Complexity & Performance Analysis

### Implementation Complexity: Medium
The rule engine leverages pre-existing parser structures:
1. **SymbolResolver**: Moderate complexity. Implements scoped name lookup and union-find equivalence mappings for SSA-lite symbols.
2. **Dominance Checker**: Low complexity. Once the CFG builder exports the dominance tree DFS intervals, dominance queries are reduced to simple integer comparisons.

### Performance Analysis
Let $V$ be the number of CFG nodes, $E$ be the number of edges, and $W$ be the number of write statements.

1. **DFS Interval Assignment**: Runs in $O(V)$ time via a single DFS pass on the Dominator Tree.
2. **Write Scan**: Runs in $O(V \times S)$ where $S$ is the maximum number of statements in a node.
3. **Dominance Queries**: Each write performs $O(F)$ checks where $F$ is the number of ownership facts. Since $F$ is typically very small ($F \le 5$), the search is virtually instant:
   $$\text{Complexity} = O(W) \text{ operations}$$
4. **Memory Footprint**: DFS interval variables consume $O(V)$ memory (two integers per node).

No heap allocations or iterative fixpoint calculations are performed during query execution, ensuring the rules engine can scan complex instructions in sub-millisecond times.

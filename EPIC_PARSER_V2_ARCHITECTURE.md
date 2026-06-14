# EPIC PARSER V2 ARCHITECTURE

**Status:** Architecture Specification
**Objective:** Design a production-grade, mathematically deterministic parser to power EPIC v1.0 as the Upgrade Intelligence Layer for Solana Programs.

---

## 1. Why does Parser V1 fail?

The V1 prototype proved the UX and CLI model, but its parsing foundation is dangerously flawed for production use.

*   **Regex Limitations:** Rust is a non-regular language. Macros, nested brackets, lifetimes, array expressions with spaces (`[u8; 32 ]`), and raw string literals instantly break regex state machines. Attempting to parse an AST with regex results in an infinite game of edge-case whack-a-mole.
*   **Namespace Collisions:** V1 maps accounts globally by `StructName`. In Solana, workspaces frequently contain multiple programs (e.g., `programs/clearing_house` and `programs/spot_margin`). If both programs define a `State` struct, V1 silently overwrites one, causing massive false negatives.
*   **Type Resolution Blindness:** V1 treats files as isolated text blobs. If `Program A` defines `pub config: ConfigStruct` but imports `ConfigStruct` from a shared `crate::state::config`, V1 cannot resolve it. It fatally defaults the size to `0 bytes`, masking catastrophic memory shifts.
*   **ABI Correctness Ignorance:** V1 diffs accounts by checking if a field name and type exist in a set. **It ignores field order.** Reordering fields in Borsh fundamentally breaks binary deserialization, yet V1 will report "Safe: No layout changes."
*   **Scalability Concerns:** V1 performs sequential file I/O and O(N^2) character-by-character comment stripping. On massive monorepos like Drift or MarginFi, this causes severe CPU blocking and memory bloat.

---

## 2. What should Parser V2 look like?

Parser V2 abandons isolated file parsing in favor of a **Semantic Project Graph**.

### Core Architecture & Data Flow

1.  **Workspace Scanner (TypeScript)**
    *   Finds `Cargo.toml` and Rust files.
    *   Reads file contents concurrently.
2.  **WASM Bridge (TypeScript <-> WASM)**
    *   Passes raw file contents and absolute paths into the WASM module.
3.  **Rust Parser (`epic-wasm` / Rust)**
    *   Uses `syn` to parse raw text into a simplified `FileAST` (Structs, Enums, Aliases, Imports).
4.  **Module Graph Builder (Rust)**
    *   Constructs the module tree by traversing `mod` and `use` declarations.
5.  **Type Resolver (Rust)**
    *   Binds local identifiers to absolute paths (e.g., `Config` -> `crate::state::config::Config`).
    *   Topologically sorts dependencies (bottom-up sizing).
6.  **Layout Engine (Rust)**
    *   Computes Borsh byte sizes and memory offsets for all resolved types.
7.  **ABI Engine (Rust)**
    *   Generates cryptographic ABI fingerprints for `#[account]` structs.
8.  **Intelligence Engine (TypeScript)**
    *   Takes the JSON diff from the WASM module.
    *   Maps cryptographic diffs to Risk Scores, Realloc Needs, and Migration Plans.

---

## 3. Rust `syn` vs Tree-sitter vs Other Approaches

| Approach | Verdict | Tradeoffs |
| :--- | :--- | :--- |
| **Regex** | **REJECTED** | Fast, but structurally incapable of understanding Rust. Dangerous. |
| **Tree-sitter** | **REJECTED** | Great CST for syntax highlighting. Fails at semantic type resolution without writing a custom compiler frontend in TypeScript. |
| **Rust Analyzer (`ra_ap_ide`)** | **REJECTED** | Perfect semantic accuracy, but wildly overkill. Extremely heavy, slow cold-starts, bloated binary. |
| **Rust `syn` (Native Binary)** | **REJECTED** | Perfect AST, but forces users to install/manage a Rust binary via Cargo or Homebrew, breaking the frictionless `npx` UX. |
| **Rust `syn` (Compiled to WASM)** | **WINNER** | Perfect AST precision. Runs everywhere Node.js runs. Zero native dependencies. |

**Why `syn` in WASM?**
`syn` is the gold standard for parsing Rust. It is what `rustc` and procedural macros use. Compiling a custom `syn`-based resolver to WASM gives us 100% Rust syntax compatibility while keeping the CLI a pure `npm` package.

---

## 4. WASM Strategy

### The Boundary
The boundary between TypeScript and Rust should be thick and declarative. Avoid chatty FFI calls.

**TypeScript Responsibilities:**
*   File system traversal and fast parallel I/O.
*   CLI UX, error formatting, and reporting.
*   CI/CD integrations and GitHub Actions formatting.

**Rust (WASM) Responsibilities:**
*   Parsing, Module Resolution, Layout Calculation, ABI Fingerprinting, and Diffing.

**The API:**
```rust
// Exposed to TypeScript via wasm-bindgen
#[wasm_bindgen]
pub fn analyze_workspace(files: JsValue) -> Result<JsValue, JsValue> { ... }

#[wasm_bindgen]
pub fn diff_workspaces(old_graph: JsValue, new_graph: JsValue) -> Result<JsValue, JsValue> { ... }
```
TypeScript passes in a `Map<FilePath, FileContent>`, and Rust returns a massive, heavily structured `WorkspaceLayout` JSON object containing all accounts, sizes, and fingerprints.

---

## 5. Type Resolution Engine

To safely calculate nested sizes, V2 must recreate the Rust compiler's name resolution logic (simplified for Borsh layouts).

1.  **Cross-File & Cross-Module Resolution:**
    *   Track `mod <name>;` declarations to build the crate tree.
    *   Track `use path::to::Type;` to map local identifiers to absolute paths.
    *   Store all types in a global `HashMap<AbsolutePath, TypeDefinition>`.
2.  **Nested Structs:**
    *   Requires a **Topological Sort**. Size primitive fields first, then nested structs, then the parent `#[account]`.
3.  **Enums:**
    *   Borsh representation: `Size = 1 byte (Discriminator) + Max(Variant Sizes)`.
4.  **Generics:**
    *   `pub struct Wrapper<T> { data: T }`. The size is indeterminate until instantiation.
    *   The engine must defer sizing until it finds `Wrapper<Pubkey>` inside an `#[account]`, then substitute `T = 32`. If `Wrapper<T>` is used without concrete types, the parser must ABORT.
5.  **Type Aliases:**
    *   `type VaultId = Pubkey;`. The resolver must seamlessly substitute `VaultId` with `32 bytes`.

---

## 6. ABI Fingerprinting V2

String comparisons are insufficient. V2 will use **Recursive Cryptographic Hashing (SHA-256)** to guarantee ABI stability.

### The Algorithm

```text
ResolvedTypeHash(Type) =
    if Primitive: Hash("Primitive" + TypeName)
    if Enum: Hash("Enum" + [Hash(VariantName + ResolvedTypeHash(VariantInner))])
    if Struct: Hash("Struct" + [Hash(FieldName + ResolvedTypeHash(FieldType))]) // Ordered!

FieldHash = Hash(FieldName + FieldOffset + ResolvedTypeHash(FieldType))

AccountFingerprint = Hash(AccountName + Discriminator + [FieldHash_1, FieldHash_2, ...])
```

### Detection Matrix
Because the hash incorporates *Offsets* and *Recursive Structure*, it perfectly detects:
*   **Field Reorder:** Offset changes -> `FieldHash` changes -> `AccountFingerprint` changes. (CRITICAL)
*   **Type Change (Same Size):** `ResolvedTypeHash` changes -> `AccountFingerprint` changes. (CRITICAL)
*   **Field Insertion:** All subsequent `FieldOffsets` change -> Cascade of hash changes. (CRITICAL)
*   **Nested Struct Change:** Inner `ResolvedTypeHash` changes -> Parent `AccountFingerprint` changes. (CRITICAL)

---

## 7. Performance Strategy

*   **Parallel File I/O:** TypeScript reads all `.rs` files concurrently using `fs.promises.readFile` before sending the batch to WASM.
*   **WASM Boundary Optimization:** Pass data into WASM once. Do not cross the FFI boundary per-file.
*   **Content-Addressable Caching:** 
    *   Hash the raw text of the `.rs` file (`SHA-256`).
    *   Store the parsed `FileAST` in a local LevelDB/SQLite cache keyed by this hash.
    *   Incremental runs only pass modified files to the `syn` parser; unmodified files are loaded instantly from cache.
*   **Memory Strategy:** The `syn` AST is massive. Extract only names, types, and imports into a lightweight `FileAST` struct, then immediately drop the `syn` tree to prevent WASM OOM crashes.

---

## 8. Testing Strategy

V2 requires a rigorous testing pyramid focused on layout correctness.

1.  **Unit Tests (Rust):** Test `syn` parsing of individual edge cases (lifetimes, complex generics, arrays with spaces, conditional compilation).
2.  **Integration Tests (Rust/TS):** Test cross-file module resolution and ABI fingerprint diffing on isolated mock workspaces.
3.  **Regression Tests (TS):** Snapshot testing of `epic-report.json` outputs for known safe and unsafe upgrades.
4.  **Real Solana Repository Fixtures:**
    EPIC must parse the following open-source workspaces without crashing or warning incorrectly:
    *   `coral-xyz/marginfi` (Massive DeFi workspace, nested structs).
    *   `drift-labs/protocol-v2` (Complex multi-program dependencies).
    *   `metaplex-foundation/mpl-token-metadata` (Native Rust, heavy macro usage).
    *   `sqds/squads-v4` (High-security multi-sig, advanced Anchor features).

---

## 9. Fail-Closed Safety Model

EPIC is infrastructure. If it cannot guarantee a layout, it must fail safely.

| Scenario | Action | Rationale |
| :--- | :--- | :--- |
| **Unknown Fixed-Size Type** | **ABORT (Exit 1)** | Guessing 0 bytes leads to fatal false negatives and mainnet halts. |
| **Dynamic Types (`String`, `Vec`)** | **WARN (Stderr) + CONTINUE** | Size is variable. Mark account as `DynamicSize` and disable strict `realloc` advice, but continue analysis. |
| **Broken Imports / Missing Files** | **ABORT (Exit 1)** | Cannot resolve type dependencies. The workspace is likely broken or incomplete. |
| **Syntax Errors** | **ABORT (Exit 1)** | Code does not compile; layout analysis is irrelevant. |
| **Unused / Broken Function Bodies** | **CONTINUE** | EPIC only cares about item definitions (structs/enums). If a function body has bad logic, ignore it. |
| **Conditional Compilation (`#[cfg]`) on fields** | **WARN (Stderr) + CONTINUE** | Alert the user that layouts may shift based on target environment. |

---

## 10. EPIC v1.0 Readiness Checklist

Before EPIC can claim **"Production Ready Upgrade Intelligence"**, the following MUST be true:

- [ ] Regex parser is completely deleted from the codebase.
- [ ] WASM-compiled `syn` parser is fully integrated and handles `npm install` gracefully.
- [ ] Cross-file module resolution successfully maps types across `mod` and `use` boundaries.
- [ ] Multi-program namespace isolation is active (Accounts are tracked as `ProgramName::AccountName`).
- [ ] Cryptographic ABI Fingerprinting accurately detects field reorders and nested structural changes.
- [ ] The Fail-Closed Safety Model is strictly enforced (No defaulting to 0 bytes).
- [ ] The CLI successfully runs end-to-end against `drift-v2` and `marginfi` without panics or OOM errors.
- [ ] The JSON output is fully documented and stable for external integrations (e.g., Squads, GitHub Actions).
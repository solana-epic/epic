# EPIC PARSER V2 - ENGINEERING AUDIT & ARCHITECTURE REVIEW

**Date:** June 14, 2026  
**Focus:** Infrastructure, Parser Integrity, ABI Stability, Safe Failure Models  
**Status:** Brutally Honest Engineering Review  

---

## 1. Infrastructure Architecture Review

The current V1 architecture (TypeScript Turborepo with Regex-based Rust parsing) is a prototype, not an infrastructure tool. It is fundamentally incapable of safely managing production deployments for Solana protocols.

### Critical Deficiencies:
*   **Regex-based AST Approximation:** Attempting to parse Rust with regular expressions guarantees failure. Rust is not a regular language. Macros, lifetimes, complex generics, nested closures, and raw string literals instantly break regex state machines.
*   **Lack of Semantic Context:** The parser treats files as isolated strings. It has no concept of Rust modules, `use` statements, or traits. If `File A` imports `MyStruct` from `File B`, the parser cannot resolve its size.
*   **Namespace Collisions:** Mapping accounts globally by name (`mapAccountsByName`) completely ignores Solana's multi-program workspace reality. This results in silent overwrites and data loss.
*   **Fatal Defaults:** Defaulting unknown type sizes to `0` instead of failing loudly is a catastrophic infrastructure anti-pattern. This will actively generate false negatives leading to corrupted on-chain state migrations.

---

## 2. Is Tree-sitter Rust the Correct Next Step?

**No. Tree-sitter is insufficient for Solana Account Layout calculation.**

Tree-sitter is a *Concrete Syntax Tree (CST)* parser. It is exceptional for syntax highlighting and simple linters, but it lacks semantic understanding. 
If Tree-sitter encounters `pub data: NestedConfig`, it only knows `NestedConfig` is a `TypeIdentifier`. It does not know:
1. Which file `NestedConfig` is defined in.
2. If `NestedConfig` is an enum or a struct.
3. If `NestedConfig` has `#[repr(C)]` or uses default Borsh serialization.

Building a semantic resolution engine (name binding, import resolution, macro expansion) on top of Tree-sitter inside TypeScript is effectively rewriting `rustc` in Node.js. It is a massive waste of engineering resources and will never be 100% accurate.

---

## 3. Parsing Strategy Comparison

| Strategy | Speed | Macro Expansion | Semantic Resolution (Sizes) | Implementation Complexity | Verdict |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Regex** | Fast | None | None | Low | **DEAD END** |
| **Tree-Sitter (TS)** | Fast | None | High (Must build in TS) | Medium | **INSUFFICIENT** |
| **Rust `syn` (CLI)** | Fast | Partial | Medium (Needs `cargo expand` or custom resolution) | Medium | **VIABLE** |
| **`ra_ap_ide` / `rust-analyzer`** | Medium | Full | Full (Exact memory layouts) | High | **OVERKILL** |
| **WASM Rust Parser (`syn` based)** | Fast | Partial | Medium (AST export to TS) | Medium | **RECOMMENDED** |

### The Winning Strategy: Hybrid WASM Rust Parser
Write the core parsing and sizing logic in a native Rust crate utilizing `syn` and `rustc-hash`. Compile this crate to WebAssembly (WASM) and execute it within the existing TypeScript Turborepo. 

**Why?**
You get the absolute correctness of Rust's official `syn` crate, the ability to safely traverse Rust module trees, and you stay within your existing Node.js CLI distribution model without requiring users to install separate binaries.

---

## 4. Scalable Architecture Recommendation (1000+ Repos)

To scale across Anchor workspaces, raw Native Rust, Codama generated clients, and future Pinocchio programs, EPIC must abandon file-by-file parsing and adopt a **Project Graph** model.

1.  **Project Discovery:** Use `cargo metadata` (invoked via child process) to map the exact dependency graph of the workspace. This natively handles `Cargo.toml` workspaces, path dependencies, and symlinks.
2.  **Semantic Graph:** Construct a Directed Acyclic Graph (DAG) of types.
    *   `Node`: Struct / Enum / Type Alias.
    *   `Edge`: "Contains" / "Depends on size of".
3.  **Borsh vs. C Layout Engine:** Solana primarily serializes using Borsh. The layout engine must explicitly calculate sizes based on Borsh rules, NOT in-memory Rust `#[repr(Rust)]` padding rules.

---

## 5. Parser Edge Cases (The Minefield)

If EPIC does not handle these natively, it will generate false positives or critical false negatives:

1.  **Constants in Arrays:** `pub data: [u8; MAX_LEN]`. EPIC must evaluate `MAX_LEN` across files.
2.  **Enums:** The size is `Discriminator (1 byte for Borsh) + Max(Variant sizes)`.
3.  **Lifetimes:** `pub authority: Pubkey<'info>`. Must be stripped during size calculation, but preserved in the ABI fingerprint.
4.  **Conditional Compilation:** `#[cfg(feature = "devnet")]`. If a struct has fields only present in devnet, EPIC must parse both or strictly require a target environment.
5.  **Type Aliases:** `type MyId = Pubkey;` If mapped blindly, it will fail to size `MyId`.
6.  **Generic Structs:** `pub struct Vault<T> { pub data: T }`. The size of `Vault` is undefined until instantiated in an `#[account]` context.
7.  **Padding & Alignment:** Native programs using `bytemuck` or `#[repr(C)]` have entirely different size calculations than Anchor/Borsh programs due to struct padding.

---

## 6. EPIC Parser V2 Architecture

### Modules & Responsibilities
*   **`epic-discover` (TS):** Invokes `cargo metadata`, finds all programs, maps `Cargo.toml` boundaries.
*   **`epic-parser-wasm` (Rust/WASM):** 
    *   Takes absolute file paths.
    *   Uses `syn` to parse Rust AST.
    *   Extracts structs, enums, type aliases, and `use` imports.
    *   Returns a JSON `RawTypeGraph`.
*   **`epic-resolver` (TS):**
    *   Takes the `RawTypeGraph` from all files.
    *   Resolves imports (e.g., binds `crate::state::Config` to the actual AST node).
    *   Performs topological sorting to ensure nested sizes are calculated bottom-up.
*   **`epic-layout` (TS):** 
    *   Applies Borsh serialization rules to the resolved types.
    *   Calculates exact byte sizes.
*   **`epic-diff` (TS):** 
    *   Compares two fully resolved layout trees.
    *   Generates the ABI Fingerprint.

### Caching Strategy
Do not cache ASTs based on file modification times (mtime). Use **Content-Addressable Hashing**.
Hash the raw text of the Rust file (`SHA-256`). Use this hash as the key in a local SQLite or LevelDB cache to store the output of the WASM parser. 

---

## 7. ABI Fingerprint System

To detect reorders, type changes, and nested mutations, we must move beyond primitive string comparison and generate a cryptographic fingerprint of the ABI.

### Hash Design
The fingerprint must be deterministic and invariant to whitespace, comments, or field names (for the binary layout), but sensitive to field names for client generation logic.

**Struct Hash Payload:**
```
Hash(
  StructName + 
  Discriminator + 
  [Field1Name, ResolvedField1TypeHash, ByteOffset] + 
  [Field2Name, ResolvedField2TypeHash, ByteOffset]
)
```
*Notice: Because `ResolvedFieldTypeHash` is recursive, changing a nested struct automatically alters the parent's hash.*

### Risk Classification Matrix
*   **Field Addition (End):** `WARNING` (Requires Realloc/Rent top-up).
*   **Field Addition (Middle):** `CRITICAL` (Binary layout shifted, existing data corrupted).
*   **Field Removal:** `CRITICAL` (Binary layout shifted, truncation risk).
*   **Field Reorder:** `CRITICAL` (Total data corruption on deserialization).
*   **Type Change (Same Size):** `CRITICAL` (e.g., `i64` to `u64`. Binary identical, semantic disaster).
*   **Type Change (Different Size):** `CRITICAL` (Layout shifted).

---

## 8. Safe Failure Model

EPIC must operate under a **Strict Fail-Closed** paradigm. Assuming sizes or ignoring unparseable blocks is unacceptable.

*   **When EPIC MUST FAIL (Process Exit 1):**
    *   An unknown type is encountered and cannot be resolved in the project graph.
    *   Cyclic dependencies are detected in struct definitions (infinite size).
    *   A generic type is used in an `#[account]` without concrete instantiation.
    *   Invalid Rust syntax blocks `syn` from generating an AST.
*   **When EPIC MUST WARN (StdErr):**
    *   Variable-length types (`String`, `Vec`, `HashMap`) are detected inside an account. EPIC must warn: `"Dynamic size detected. EPIC assumed 4-byte prefix. Realloc analysis is disabled for this account."`
    *   Conditional compilation flags (`#[cfg(...)]`) are detected on account fields.
*   **When EPIC MUST CONTINUE:**
    *   Unrelated code (functions, tests, traits) fails to parse or resolve. (EPIC only cares about state layouts).

---

## 9. Engineering Roadmap

### Phase 1: Parser Trustworthiness (Weeks 1-3)
*   Burn the Regex parser.
*   Implement `epic-parser-wasm` using `syn`.
*   Implement the topological sort for nested struct sizing.
*   Enforce the Strict Fail-Closed model. (Return `Result::Err`, never `0 bytes`).

### Phase 2: ABI Intelligence (Weeks 4-5)
*   Implement ABI Fingerprinting (Recursive Type Hashing).
*   Detect Reorders, Type Swaps, and offset shifting.
*   Update diff engine to output the Risk Matrix accurately.

### Phase 3: Migration Intelligence (Weeks 6-7)
*   Generate safe `realloc` snippets based on exact byte diffs.
*   Flag when `realloc` is unsafe (e.g., field inserted in the middle of a struct).

### Phase 4: Bankrun Simulation (Weeks 8-9)
*   Integrate `solana-bankrun`.
*   Generate a test script that clones mainnet accounts, applies the ABI changes, and tests the deserialization boundary to empirically prove migration safety.

### Phase 5: CI/CD & GitHub Actions (Week 10)
*   Wrap EPIC inside a Dockerized GitHub Action.
*   Post automated PR comments blocking merges if `CRITICAL` ABI breaks are detected without matching migration logic.

---

## 10. Production Blockers & Highest Leverage Output

**What would block approval today?**
The regex parser, the inability to process multi-program namespaces, the fallback to `0 bytes` for unknown types, and the complete blindness to field reordering. As it stands, the tool is a hazard to protocol safety.

**What must be fixed first?**
Delete `sizeOfRustType`'s fallback to `0`. If a type is unknown, the program must throw a fatal error. 

**What should never be built?**
Do not build "Automated Migration Code Generators". Do not try to write Rust code that automatically shifts bytes for the developer. State migrations are highly protocol-specific. Provide intelligence and warnings, but never mutate the source of truth for protocol state.

**What is the single highest leverage engineering improvement?**
**The WASM-compiled `syn` parser connected to a recursive topological type resolver.** Once you possess an exact, 100% reliable graph of how every struct maps to bytes in Borsh, every other feature (diffing, simulations, security warnings) becomes mathematically trivial. Get the foundation right.

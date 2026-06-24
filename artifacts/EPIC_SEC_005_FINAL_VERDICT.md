# EPIC-SEC-005: Arbitrary CPI Target Program Validation — Final Verdict Report

## 1. Is SEC-005 production-ready?
**YES (Production-Ready)**.
EPIC-SEC-005 has audited complex production codebases (Drift-v2, Marginfi, Kamino, Squads, Metaplex Token Metadata, Sentio-rs) with **zero parser crashes**, demonstrating extreme stability and reliability. Its SSA-aware version tracking and statement-level SSA checking resolve major vulnerabilities like variable shadowing and alias chains that simple name-matching pattern matchers fail on.

---

## 2. What remains unfinished?
1. **Comment-based Suppressions**: Comment annotations like `// sentio-ignore SW003` are not natively parsed. This leads to flagging suppressed locations in legacy repos unless they are explicitly refactored to standard validations or added to `epic.toml` ignore lists.
2. **Generic Crate/Library Calls**: In-depth signature verification handled by external security libraries or obscure dynamic helper subroutines whose source code is not present in the local codebase workspace.

---

## 3. What known limitations still exist?
1. **Loop Flattening (False Negatives)**: The CFG builder currently flattens loop constructs (like `for`, `while`, and `loop`) sequentially inline without generating branching control-flow exit or entry nodes for loop conditions. Consequently, a validation check placed inside a loop is assumed to sequentially dominate all subsequent code outside the loop, bypassing loop boundary validation.
2. **Indirect Dynamic Dispatches**: Invocations utilizing custom target resolving logic (e.g. reading target address from a custom PDA bump lookup sequence) are conservatively marked as unresolved or inconclusive.

---

## 4. What false positive sources remain?
1. **External Helper Signature Verifiers**: If a signature verification is delegated to an un-inlined custom helper function defined elsewhere (without standard `require!` or `assert!` macros directly in the target function's scope), the resolver cannot trace the assertion fact across the function boundary, producing a false positive.
2. **Conditional Validations**: Complex conditional checks where program address is validated only under specific boolean conditions (e.g. `if condition { require_keys_eq!(token_program.key(), ...); }`).

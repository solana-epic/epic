# EPIC Competitive Positioning

A founder-friendly comparison of Solana program verification tooling.

| Feature / Capability | Sentio | Anchor Sentinel | EPIC |
| :--- | :---: | :---: | :---: |
| **Security Rules Audit** | ❌ (Linters only) | ❌ (Metadata only) | ✅ (EPIC-SEC-001 to 005) |
| **Upgrade Safety Verification** | ❌ | ⚠️ (IDL Diffs) | ✅ (Direct AST Byte-Layouts) |
| **CFG (Control Flow Graphs)** | ❌ | ❌ | ✅ (Path-Sensitive Splits) |
| **SSA (Single Static Assignment)** | ❌ | ❌ | ✅ (Variable Versioning) |
| **Rust AST Deep Parser** | ❌ | ❌ | ✅ (Full Syn-based analysis) |
| **SARIF Output** | ❌ | ❌ | ✅ (GitHub Scan native) |
| **Real Repository Validation** | ❌ | ❌ | ✅ (Drift, Kamino, Marginfi, etc.) |

### Why EPIC?
*   **Traditional Linters (e.g. Sentio)** use basic regex/syntax pattern matching, resulting in high false positives and bypasses.
*   **Anchor Sentinel** only compares metadata IDLs and ignores instruction code execution logic.
*   **EPIC** is a compiler-powered analysis engine, building CFGs, tracing variables via SSA, and asserting security invariants on source code directly.

# EPIC Configuration System: `epic.toml` Design & Specification

To prevent CI noise while maintaining a strict safety boundary for mainnet deployments, EPIC utilizes a local configuration file named `epic.toml`. This document details the exact configuration design, answers the 10 core questions, establishes validation rules, and presents real-world protocol configurations.

The full, detailed specification has been written to [EPIC_CONFIG_SPEC.md](file:///Users/aksh/Documents/Solana%20EPIC/EPIC_CONFIG_SPEC.md). Below is a summary of the core design decisions.

---

## 1. Core Design Decisions Summary

| Question | Core Decision / Rule |
| :--- | :--- |
| **1. Configuration Options** | Global compare settings, exclude paths, RPC settings, and program-level override rules. |
| **2. Allowable Ignores** | Trailing field additions, padding repurposing (size-neutral), and enum variant additions. |
| **3. Non-Ignorable Findings** | `FIELD_REORDERED`, `FIELD_REMOVED`, and `TYPE_CHANGED` (non-trailing width change). |
| **4. Severity Overrides** | Surgical: Scoped mapping (`program`, `account`, `field`, `finding`) with mandatory audit note. |
| **5. Monorepo Config** | Hierarchical: Workspace-level config at root with local overrides inside program folders. |
| **6. CI Environment** | Auto-discovers `epic.toml`. Fails on invalid overrides or missing notes. Warns if `epic.toml` is modified in PR. |
| **7. Real-world Protocol Profiles** | **Drift**: Appending trailing fields. **Marginfi**: Padding repurposing. **Kamino**: IDL comparison mode. **Squads**: Zero-override lock. **Mango**: Folder exclusion patterns. |
| **8. Security Blind Spots** | Wildcards (`*`), stale/orphaned overrides, permissive gating, and path exclusion abuse. |
| **9. Developer Bypass Prevention** | The Unbreakables (hardcoded block on reordering/removals), note character limits, strict PR flags. |
| **10. exact v0.1 Schema** | Strongly-typed TOML format with explicit overrides array. |

---

## 2. The v0.1 `epic.toml` Schema

```toml
[workspace]
compare_mode = "ast"          # Options: "ast" (Rust AST parser) or "idl" (Anchor IDL JSON)
fail_on_severity = "MAJOR"    # Options: "SAFE", "MINOR", "MAJOR", "CRITICAL"
rpc_url = "https://api.mainnet-beta.solana.com"
exclude_paths = [
    "**/tests/**",
    "**/mocks/**",
    "**/target/**"
]
enforce_padding = true

[programs]
marginfi = { path = "./programs/marginfi", id = "MFv28xrwG2k1GZnhwYhcz1GL9G7gW4mh99PP5zER6NL", idl_path = "./target/idl/marginfi.json" }
drift = { path = "./programs/drift", id = "dRifv2G2XadHceee5mK3dB6vJ61g2QskXn8o1sBDR1B", idl_path = "./target/idl/drift.json" }

[[programs.marginfi.overrides]]
account = "Bank"
finding = "PADDING_REPURPOSE"
field = "reserved"
action = "allow"
note = "Replaced reserved: [u8; 64] with new_stat: u64 and reserved: [u8; 56]. Layout verified."
```

---

## 3. Recommended Implementation Plan

1.  **Phase 1: TOML Parser Integration (Day 1)**: Install the `toml` package in `@epic/parser` and load `epic.toml` into a strongly-typed `EpicConfig` interface.
2.  **Phase 2: Strict Validation Engine (Day 2)**: Check the config at runtime for illegal overrides (e.g. attempting to override `FIELD_REORDERED`), wildcards (`*`), and short or missing audit notes.
3.  **Phase 3: Override Resolution Logic (Day 3)**: Update `compareAccountLayouts` in `@epic/diff-engine` to apply configured overrides, downgrading severities accordingly.
4.  **Phase 4: CI Integration & Stale Checks (Day 4)**: Enhance the GitHub Action to warn on stale or modified `epic.toml` files in the PR comment.

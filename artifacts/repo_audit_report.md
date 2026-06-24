# Solana-EPIC: Repository Audit Report

This report evaluates the current state of the Solana-EPIC repository root, documentation files, and the `README.md` to identify developer experience gaps, obsolete internal artifacts, and layout adjustments needed to transition this project into a public-facing developer tool.

---

## 1. README Analysis

### What Information Exists Today
* **Core Definition & Purpose**: Explains that EPIC is an upgrade intelligence layer for verifying Solana state layouts and preventing account serialization crashes.
* ** Solana Upgrade Problem Context**: Briefly covers how Borsh flat byte serialization shifts offsets and why this leads to critical errors.
* **Basic Installation & Command Structure**: References `epic analyze` and `epic check` commands.
* **Basic GitHub Action Integration**: Includes a simple workflow snippet.
* **Historical Validation Summary**: Contains a table showing five major historical protocol upgrade events (Drift, Kamino, Mango, MarginFi, Squads) where EPIC successfully identified risks.
* **Basic Roadmap & References**: Outlines near-term versions and lists documentation links.

### What Information is Missing
* **High-Level Flow/Architecture Visualization**: There is no diagram mapping the compiler front-end parser to the rule engine (AST -> CFG -> SSA -> GuardFacts -> Rules -> CLI/SARIF).
* **Detailed Rules Breakdown**: No explicit mention of `EPIC-SEC-001` (Owner Validation) or WDG (Write-Dependency Graph) tracking.
* **Deterministic vs. Heuristic Philosophy**: Doesn't emphasize the core differentiator—EPIC evaluates deterministic compiler semantic paths rather than simple pattern-matching regex/linter rules.
* **Actual Workspace Commands**: References to `epic check` are generic and lack concrete paths or instructions on how to run a diff scan on actual workspaces.
* **Accurate Roadmap & Package Structure**: No clear directory layout walkthrough explaining how the workspace is structured (e.g. `packages/cli`, `packages/parser-v2`, `packages/diff-engine`, `packages/github-action`).

### Visitor Context Gaps

#### First-Time Visitors
* **Confusing**: The CLI usage lists `epic analyze` and `epic check` without clearly showing how these map to packages in the repository. A visitor looking at the root package structure wouldn't know how to run these commands locally.
* **Friction**: The root contains 40+ raw markdown files (`EPIC_PARSER_V3_DESIGN_REVIEW.md`, `epic_sec_001_pre_implementation_review.md`, etc.), which look like internal planning drafts/scratches rather than a curated release.

#### Grant Reviewers
* **Confusing**: Grant reviewers want to see immediate proof of technical feasibility and design robustness. The current README refers to "Anchor IDL 0.30+ spec, overrides" but does not cleanly connect them to the architectural blocks (SSA-lite, GuardFacts, WDG).
* **Missing Value**: The unique differentiator—deterministic static compiler checks over standard linters—is buried.

#### Protocol Engineers
* **Confusing**: Protocol engineers need to know *exactly* what files/directories are scanned, how the parser handles nested scopes, shadowing, and aliases, and what constitutes a SAFE/UNSAFE upgrade. The current README lacks concrete CLI execution examples on real directories.

---

## 2. Repository Root Audit

### Obsolete / Internal-Only Artifacts
These files are internal specifications, hostile review documents, implementation specs, and task lists. They clutter the root directory and must be relocated to `docs/archive/`:
* `EPIC_BENCHMARK_REPORT.md`
* `EPIC_CAPABILITY_AUDIT.md`
* `EPIC_DOCUMENT_RETENTION_PLAN.md`
* `EPIC_GUARDFACT_FINAL_ARCHITECTURE.md`
* `EPIC_PARSER_CAPABILITY_AUDIT.md`
* `EPIC_PARSER_V3_BUILD_SEQUENCE.md`
* `EPIC_PARSER_V3_DESIGN_REVIEW.md`
* `EPIC_PARSER_V3_FINAL_ARCHITECTURE.md`
* `EPIC_PARSER_V3_IMPLEMENTATION_PLAN.md`
* `EPIC_PARSER_V3_TASKLIST.md`
* `EPIC_PHASE1_REDESIGN.md`
* `EPIC_PHASE1_SECURITY_ARCHITECTURE.md`
* `EPIC_REAL_WORLD_VALIDATION.md`
* `EPIC_REPOSITORY_HEALTH.md`
* `EPIC_SECURITY_ENGINE_STATUS.md`
* `EPIC_SEC_001_IMPLEMENTATION_REDESIGN.md`
* `EPIC_TEST_MATRIX.md`
* `EPIC_VALIDATION_GAPS.md`
* `EXTERNAL_TESTING_PLAN.md`
* `ISSUE_1_IMPLEMENTATION_SPEC.md`
* `ISSUE_2_FINAL_IMPLEMENTATION_SPEC.md`
* `ISSUE_2_IMPLEMENTATION_SPEC.md`
* `ISSUE_2_PRE_IMPLEMENTATION_REVIEW.md`
* `PILOT_RECRUITMENT_PACK.md`
* `PRIVATE_BETA_EXECUTION_PLAN.md`
* `README_V2.md`
* `epic_guardfact_hostile_architectural_review.md`
* `epic_guardfact_hostile_validation_audit.md`
* `epic_issue_5_anchor_constraints_spec.md`
* `epic_issue_5a_guardfact_core_model_spec.md`
* `epic_sec_001_final_hostile_architecture_signoff.md`
* `epic_sec_001_implementation_plan.md`
* `epic_sec_001_owner_validation_spec.md`
* `epic_sec_001_pre_implementation_review.md`

### Duplicate / Stale Documentation in `docs/`
These files are duplicates or internal milestone logs that should be archived:
* `docs/GRANT-NOTES.md` -> `docs/archive/`
* `docs/MILESTONES.md` -> `docs/archive/`
* `docs/PARSER_V2_SPIKE.md` -> `docs/archive/`
* `docs/VISION.md` -> `docs/archive/`

### Files to Remain in Root
* `README.md` (Main entry point, to be rewritten)
* `CHANGELOG.md` (Release history)
* `CONTRIBUTING.md` (Contributor setup guide)
* `package.json` & `package-lock.json` (NPM workspace definitions)
* `epic.toml` (Default analyzer configuration rules)
* `.gitignore` (Repository tracking hygiene)

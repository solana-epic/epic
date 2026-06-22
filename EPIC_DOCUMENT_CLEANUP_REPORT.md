# EPIC Document Cleanup Report

This report defines the classification and plan of action for auditing the repository's markdown documents and generated artifacts before the public release.

## Classification Matrix

| File Path | Classification | Target Path / Action | Reason |
| :--- | :---: | :--- | :--- |
| `README.md` | **KEEP** | Retain in root | Core public-facing introduction and installation guide. |
| `CHANGELOG.md` | **KEEP** | Retain in root | Tracks project version history. |
| `CONTRIBUTING.md` | **KEEP** | Retain in root | Code contribution guidelines for open-source developers. |
| `EPIC_PERFORMANCE_REPORT.md` | **KEEP** | Move to `docs/performance-report.md` | Performance and sizing benchmark statistics valuable to reviewers. |
| `EPIC_SENTINEL_COMPARISON.md` | **KEEP** | Move to `docs/sentinel-comparison.md` | Comparison guide detailing EPIC's static analysis features vs. heuristic scanners. |
| `EPIC_BLOCK_TRAVERSAL_REPORT.md` | **ARCHIVE** | Move to `docs/archive/EPIC_BLOCK_TRAVERSAL_REPORT.md` | Temporary stabilization report for Task 3. |
| `EPIC_SSA_SCOPE_FIX_REPORT.md` | **ARCHIVE** | Move to `docs/archive/EPIC_SSA_SCOPE_FIX_REPORT.md` | Temporary stabilization report for Task 4. |
| `EPIC_WDG_ASSIGNMENT_FIX_REPORT.md` | **ARCHIVE** | Move to `docs/archive/EPIC_WDG_ASSIGNMENT_FIX_REPORT.md` | Temporary stabilization report for Task 5. |
| `EPIC_TYPE_UNPACKING_FIX_REPORT.md` | **ARCHIVE** | Move to `docs/archive/EPIC_TYPE_UNPACKING_FIX_REPORT.md` | Temporary stabilization report for Task 1. |
| `EPIC_IMPERATIVE_CHECK_REPORT.md` | **ARCHIVE** | Move to `docs/archive/EPIC_IMPERATIVE_CHECK_REPORT.md` | Temporary stabilization report for Task 2. |
| `EPIC_POST_FIX_VALIDATION.md` | **ARCHIVE** | Move to `docs/archive/EPIC_POST_FIX_VALIDATION.md` | Post-fix verification results and exploit detection status. |
| `EPIC_HISTORICAL_VALIDATION.md` | **ARCHIVE** | Move to `docs/archive/EPIC_HISTORICAL_VALIDATION.md` | Legacy historical exploit testing metrics. |
| `REAL_WORLD_AUDIT_VALIDATION.md` | **ARCHIVE** | Move to `docs/archive/REAL_WORLD_AUDIT_VALIDATION.md` | Early real-world repository audit audit logs. |
| `EPIC_PARSER_V2_REVIEW.md` | **ARCHIVE** | Move to `docs/archive/EPIC_PARSER_V2_REVIEW.md` | Code review and structural gap analysis of parser v2. |
| `EPIC_STRATEGY_REVIEW.md` | **ARCHIVE** | Move to `docs/archive/EPIC_STRATEGY_REVIEW.md` | Project milestone planning and competitive assessment. |
| `EPIC_V2_VALIDATION_REPORT.md` | **ARCHIVE** | Move to `docs/archive/EPIC_V2_VALIDATION_REPORT.md` | General integration verification findings. |
| `EPIC_FALSE_POSITIVE_REPORT.md` | **ARCHIVE** | Move to `docs/archive/EPIC_FALSE_POSITIVE_REPORT.md` | Early metrics on false alarm frequencies. |
| `EPIC_PRODUCTION_READINESS_REPORT.md`| **ARCHIVE** | Move to `docs/archive/EPIC_PRODUCTION_READINESS_REPORT.md` | Internal audit for production gates. |
| `EPIC_EDGE_CASE_BREAKAGE_REPORT.md` | **ARCHIVE** | Move to `docs/archive/EPIC_EDGE_CASE_BREAKAGE_REPORT.md` | Breakage logs and failure boundary tests. |
| `EPIC_SARIF_VALIDATION.md` | **ARCHIVE** | Move to `docs/archive/EPIC_SARIF_VALIDATION.md` | Testing report on SARIF specification compatibility. |
| `EPIC_PARSER_V2_ARCHITECTURE.md` | **ARCHIVE** | Move to `docs/archive/EPIC_PARSER_V2_ARCHITECTURE.md` | Specification detailing AST and type resolver design. |
| `idea-context.md` | **ARCHIVE** | Move to `docs/archive/idea-context.md` | Colosseum Landscape Analysis and competitive positioning document. |
| `packages/github-action/DESIGN.md` | **ARCHIVE** | Move to `docs/archive/github-action-design.md` | Initial architecture and execution details for the GitHub Action. |
| `epic-report.md` | **DELETE** | Remove | Generated temporary markdown report from CLI demo run. |
| `epic-report.json` | **DELETE** | Remove | Generated temporary JSON report from CLI run. |
| `sarif.json` | **DELETE** | Remove | Generated temporary SARIF artifact. |

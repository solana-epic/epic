# EPIC Roadmap

## Phase 1: Parser and Upgrade Diff Foundation

Goal: prove EPIC can understand Anchor account layouts and explain upgrade impact.

Scope:

- Anchor account parser.
- Account byte-size calculation.
- Account diff engine.
- `epic analyze <path>`.
- `epic check <old_path> <new_path>`.
- Human-readable upgrade readiness report.

Phase 1 success means EPIC can detect layout changes and produce practical migration recommendations from local source code.

## Phase 2: SHIFT State Migration Intelligence

Goal: move from detecting layout changes to planning migration work.

Scope:

- More complete Rust and Anchor type sizing.
- Variable-length account handling through explicit annotations or configuration.
- Migration requirement classification.
- Field default-value guidance.
- Reallocation and rent impact estimates.
- IDL change awareness.

Phase 2 success means EPIC can produce a migration plan that a Solana engineer can act on before deployment.

## Phase 3: Solana CI/CD Intelligence

Goal: make upgrade readiness part of normal Solana release workflows.

Scope:

- GitHub Actions integration.
- Risk thresholds and CI gates.
- Pull request reporting.
- Release readiness artifacts.
- SolDeploy readiness checks.

Phase 3 success means Solana teams can block, warn, or approve upgrades based on structured deployment intelligence in CI.

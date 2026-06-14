# EPIC Architecture

EPIC is a TypeScript Turborepo with separate packages for the CLI and reusable analysis logic.

## High-Level Design

The system is intentionally modular:

- CLI package: command-line interface and human-readable report formatting.
- Parser package: Anchor project discovery, Rust account parsing, byte-size calculation, account diffing, and upgrade readiness primitives.
- Future modules: SHIFT for state migration intelligence and SolDeploy for deployment readiness workflows.

The CLI should stay thin. Analysis rules belong in reusable packages so they can later power CI, GitHub Actions, and other integrations.

## CLI

The CLI is implemented with Commander.js.

Current commands:

- `epic analyze <path>` scans a Rust file or project directory and prints Anchor account names with calculated byte sizes.
- `epic check <old_path> <new_path>` compares two project versions and prints an upgrade readiness report.

The CLI owns formatting, argument parsing, and process exit behavior. It does not own parsing or decision logic.

## Parser

The parser package currently handles:

- Walking Rust source trees.
- Detecting `#[account]` and `#[account(...)]` structs.
- Extracting named fields.
- Calculating Anchor account byte sizes, including the 8-byte discriminator.
- Returning structured account definitions for downstream analysis.

The parser is intentionally conservative. Unsupported or variable-length types should surface as explicit notes as the sizing model becomes more complete.

## Diff Engine

The diff engine compares old and new account definitions by account name and field name.

It detects:

- Added accounts.
- Removed accounts.
- Added fields.
- Removed fields.
- Type changes.
- Account size changes.

It produces reusable `AccountDiff` objects that include migration requirement, risk level, and recommended actions.

## SHIFT Module

SHIFT will be the state migration analysis layer. It should build on parser and diff outputs to answer:

- Which account layouts require migration?
- Can existing accounts be reallocated safely?
- Which fields need default values?
- Which transformations should be handled in instructions or scripts?

SHIFT should not deploy programs. It should produce migration intelligence and developer-facing plans.

## SolDeploy Module

SolDeploy will be the deployment readiness layer. It should combine account diffs, IDL changes, client impact, migration plans, and preflight checks into a release decision.

Initial SolDeploy responsibilities may include:

- Readiness reports.
- Deployment checklists.
- CI gates.
- Release artifacts.

## Future GitHub Actions Integration

GitHub Actions should run EPIC automatically on pull requests and release branches.

Expected flow:

1. Checkout old and new program versions.
2. Run `epic check <old_path> <new_path>`.
3. Attach the report to CI logs or pull request comments.
4. Fail or warn based on configured risk thresholds.

The current CLI and parser boundaries are designed so this integration can call the same decision engine without duplicating logic.

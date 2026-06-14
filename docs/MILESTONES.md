# EPIC Milestones

## MVP Milestone 1: Anchor Account Analysis

Deliverables:

- Parse Anchor projects from local paths.
- Detect `#[account]` structs.
- Extract account fields.
- Calculate account byte sizes.
- Print account names and sizes through `epic analyze`.

Success metrics:

- Works against simple Anchor programs.
- Produces deterministic output.
- Has parser test coverage for common account field types.

## MVP Milestone 2: Upgrade Diff Engine

Deliverables:

- Compare old and new account definitions.
- Detect field additions, removals, type changes, and size changes.
- Produce structured diff output from the parser package.
- Add tests for common layout-change scenarios.

Success metrics:

- Correctly identifies migration-relevant account changes.
- Distinguishes low, medium, and high risk changes.
- Can be reused outside the CLI.

## MVP Milestone 3: Upgrade Readiness Report

Deliverables:

- Add `epic check <old_path> <new_path>`.
- Print human-readable upgrade readiness reports.
- Include migration requirement, risk level, and recommended actions.
- Include overall risk.

Success metrics:

- Report is understandable without reading source code.
- Recommendations are concrete enough to guide next engineering steps.
- Output is stable enough for CI logs.

## MVP Milestone 4: CI Readiness

Deliverables:

- Define machine-readable report output.
- Add configurable risk thresholds.
- Prepare GitHub Actions usage documentation.
- Add tests around report generation.

Success metrics:

- Can run in CI without special environment assumptions.
- Can fail a job on high-risk upgrades.
- Produces useful logs for pull requests.

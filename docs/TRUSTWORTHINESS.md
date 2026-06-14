# EPIC Trustworthiness

EPIC v0.4 is a correctness release. It hardens the parser and upgrade engine so developers can trust the output before using EPIC in higher-level migration, CI, or deployment workflows.

## Why These Checks Exist

Upgrade intelligence is only useful if layout analysis is conservative. A wrong account size, missed field reorder, or account-name collision can produce advice that looks precise but is unsafe.

The v0.4 checks exist to prevent EPIC from silently producing misleading upgrade plans.

## What EPIC Guarantees

EPIC now guarantees:

- Unknown fixed-size types do not default to zero bytes.
- Unknown fixed-size types abort analysis with a fatal error.
- Fatal type errors identify the account, field, type, and file path.
- Account matching is namespace-aware using source path plus account name.
- ABI fingerprints include account name, ordered field names, and ordered field types.
- Field reorders are detected as ABI changes and marked critical.
- Dynamic containers are flagged with warnings.
- Dynamic layouts are never presented as exact static realloc analysis.

## What EPIC Does Not Guarantee

EPIC does not yet guarantee:

- Full Rust type resolution across modules or crates.
- Exact sizing for user-defined nested structs.
- Exact runtime rent exemption lamports.
- Runtime transaction simulation.
- Anchor macro expansion.
- Validation against live chain state.

When EPIC cannot determine a layout safely, it should stop or warn instead of guessing.

## Safe Failure Philosophy

EPIC follows a fail-closed model for layout safety:

- If a fixed-size type is unknown, analysis aborts.
- If a dynamic-size type is known, analysis continues with a warning.
- If ABI layout changes without size changing, EPIC still reports the change.
- If account names collide across programs, EPIC treats them as separate accounts.

This is stricter than prototype behavior, but it is necessary for upgrade intelligence.

## Future Parser Roadmap

The parser should evolve toward:

- Module-aware Rust parsing.
- User-defined struct resolution.
- Anchor IDL ingestion as a secondary source of truth.
- Better source locations with line and column numbers.
- Configurable type aliases.
- Exact dynamic account sizing through developer-provided bounds.
- RPC rent exemption lookup.
- Runtime simulation through a future Bankrun adapter.

None of those should weaken the current fail-closed behavior.

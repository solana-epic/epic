# EPIC Grant Notes

## Solana Foundation Positioning

EPIC supports safer Solana program upgrades by giving developers deployment intelligence before they ship. It focuses on operational correctness: account layout changes, migration requirements, rent impact, IDL regeneration, and client rebuilds.

This matters because Solana programs often manage long-lived state. A small account layout change can require migration planning across existing accounts. EPIC helps teams catch that class of issue earlier in the development lifecycle.

EPIC is infrastructure for developer confidence, not a replacement for audits or security tools.

## Superteam Positioning

For Superteam builders, EPIC is a practical CLI-first tool that improves shipping discipline. It helps small teams get upgrade readiness feedback without setting up a large platform or relying on manual review.

The project is aligned with the way Solana teams actually build:

- Anchor-first.
- CLI-first.
- CI-friendly.
- Focused on fast iteration and production readiness.

## Why This Matters to Solana Developers

Solana developers need to move quickly without breaking existing state. Upgrade risk is not limited to security vulnerabilities. It also includes layout drift, account resizing, client incompatibility, and missing migration steps.

EPIC makes those risks explicit:

- It shows what changed.
- It explains whether migration is required.
- It assigns a practical risk level.
- It recommends next actions before deployment.

The long-term value is a stronger Solana deployment workflow where every upgrade can be checked, reviewed, and shipped with clearer operational context.

# EPIC

> **Upgrade Intelligence for Solana Programs.**  
> *Know what changes. Know what breaks. Ship with confidence.*

---

EPIC is the deployment readiness and upgrade intelligence infrastructure for Solana programs. Positioned between git push and mainnet, EPIC evaluates account layout evolution, ABI compatibility, and security regressions before upgrades reach production.

## Why Upgrade Intelligence Matters

Every Solana program upgrade is a high-risk mutation of mainnet state. While standard compiler tooling validates syntactic correctness, it cannot detect layout shifts, offset drifts, or missing cache reloads that corrupt existing account data.

Most protocol teams discover breaking layout changes and security regressions after deployment. EPIC provides the compile-time visibility needed to verify upgrade safety beforehand.

## Core Capabilities

*   **Upgrade Compatibility Checking**: Compare program versions to detect state layout drift, field reordering, type width changes, and Anchor discriminator shifts.
*   **Account Evolution Metrics**: Analyze serialized account sizes, offset structures, and memory growth impact to manage state scaling.
*   **Upgrade Safety Verification**: Audit safety constraints, signer checks, and post-CPI reload invariants introduced during upgrade changes.
*   **Deployment Readiness Pipeline**: Integrate upgrade validation into CI/CD workflows to prevent breaking layouts from reaching mainnet.

## Installation

Install the command-line interface:

```bash
npm install -g @solana-epic/cli
```

## Repositories

*   **[epic](https://github.com/solana-epic/epic)** — Core CLI, compiler-model layout diffing engine, and GitHub Action integration.
*   **[epic-web](https://github.com/solana-epic/epic-web)** — Dashboard interface for tracking upgrade history and deployment readiness metrics.

## Roadmap

*   [ ] IDL-based layout drift and state migration validation
*   [ ] Interactive CLI layout visualization and diffing tools
*   [ ] Local editor LSP diagnostics for real-time layout feedback
*   [ ] Automated state migration helper generation

## Contributing

Review the contribution guidelines in the core repository to get started.

---

*EPIC is open-source developer tooling licensed under the MIT License.*

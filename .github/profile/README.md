# EPIC

EPIC is a security-first upgrade intelligence platform for Solana programs. It provides protocol teams with compiler-level static analysis to detect upgrade risks, analyze account layout changes, validate state migrations, and audit security vulnerabilities before code is deployed.

## Core Capabilities

*   **Upgrade Compatibility Analysis**: Detect state layout drift, field reordering, type size changes, and Anchor discriminator shifts between program versions.
*   **Static Security Auditing**: Verify account ownership validations, transaction signer checks, post-CPI state reloads, and PDA seed derivation safety invariants.
*   **Account Layout Verification**: Compute serialized struct sizes, alignment offsets, and memory footprints to manage state growth impact.
*   **Automated CI/CD Integration**: Upload SARIF-compliant findings directly to GitHub Code Scanning pipelines for inline PR annotations.

## Installation

Install the command-line interface globally via npm:

```bash
npm install -g @solana-epic/cli
```

## Resources

*   **Core Repository**: [github.com/solana-epic/epic](https://github.com/solana-epic/epic)
*   **Releases**: [github.com/solana-epic/epic/releases](https://github.com/solana-epic/epic/releases)
*   **Documentation**: [github.com/solana-epic/epic#readme](https://github.com/solana-epic/epic#readme)

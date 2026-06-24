# EPIC Package Size & Hygiene Audit

This audit evaluates the content and size of the generated npm tarballs to minimize installation footprint and prevent code leakage.

## 1. Whitelist Whitelist Check

Prior to this audit:
*   `@solana-epic/cli`, `@solana-epic/parser`, and `@solana-epic/diff-engine` correctly restricted published files using the `"files": ["dist"]` configuration.
*   `@solana-epic/github-action` did **not** specify a `"files"` whitelist, meaning `npm pack` included development source files, configuration files, and unit tests.
*   The prebuilt binary packages (`@solana-epic/cli-...`) correctly whitelisted only the native compiled `parser-v2` executables under `bin/`.

---

## 2. Size Comparison & Remediation

By whitelisting `dist` and `action.yml` in `@solana-epic/github-action`, we successfully reduced its unpacked size:

| Package | Original Size | Whitelisted Size | Footprint Reduction |
| :--- | :--- | :--- | :--- |
| `@solana-epic/github-action` | 1.3 MB (18 files) | **1.22 MB (4 files)** | **-6% unpacked files/metadata size** |

---

## 3. Hygiene Compliance

The whitelists prevent the following leakage:
*   No `.md` files or archived reports are packaged (except `README.md` at root).
*   The `artifacts/` and `brain/` folders are strictly excluded.
*   Rust source code (`parser-v2/src/`) and historical test fixtures are excluded.
*   Only prebuilt platform targets package binary files.

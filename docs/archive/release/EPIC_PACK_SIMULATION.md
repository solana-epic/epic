# EPIC Packaging Simulation Report

This report documents the simulation of npm publication using `npm pack` across all 8 publishable workspaces.

## Packed Tarballs Summary

All packages were compiled and packed successfully. The generated `.tgz` files are stored in `artifacts/packages/`:

| Package Name | Tarball File | Packed Size | Unpacked Size | File Count |
| :--- | :--- | :---: | :---: | :---: |
| **`@epic/cli-darwin-arm64`** | `epic-cli-darwin-arm64-0.1.0-beta.1.tgz` | 1.25 MB | 3.4 MB | 2 |
| **`@epic/cli-darwin-x64`** | `epic-cli-darwin-x64-0.1.0-beta.1.tgz` | 1.22 MB | 3.3 MB | 2 |
| **`@epic/cli-linux-x64`** | `epic-cli-linux-x64-0.1.0-beta.1.tgz` | 1.29 MB | 3.7 MB | 2 |
| **`@epic/cli-win32-x64`** | `epic-cli-win32-x64-0.1.0-beta.1.tgz` | 1.85 MB | 5.1 MB | 2 |
| **`@epic/parser`** | `epic-parser-0.1.0-beta.1.tgz` | 29.3 kB | 154.2 kB | 49 |
| **`@epic/diff-engine`** | `epic-diff-engine-0.1.0-beta.1.tgz` | 9.6 kB | 42.7 kB | 25 |
| **`@epic/cli`** | `epic-cli-0.1.0-beta.1.tgz` | 15.6 kB | 73.2 kB | 13 |
| **`@epic/github-action`** | `epic-github-action-0.1.0-beta.1.tgz` | 260.6 kB | 1.3 MB | 18 |

---

## Contents Verification

1.  **Platform Packages (`@epic/cli-*`)**:
    *   Tarballs contain only `package.json` and the prebuilt binary executable (`bin/parser-v2` or `bin/parser-v2.exe`).
    *   No debug symbols or raw compilation cache assets were bundled.
2.  **Core Packages**:
    *   Tarballs contain compiled Javascript files (`dist/`) and declarations (`.d.ts`).
    *   Unnecessary developer assets (`src/`, `tsconfig.json`, tests) were excluded from `@epic/parser`, `@epic/diff-engine`, and `@epic/cli` packages via the `files` directive in their respective `package.json` descriptors.

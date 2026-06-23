# EPIC Packaging Audit

This document evaluates the packaging and installability of the EPIC monorepo for developers and general users.

## Audit Answers

### 1. Can a brand-new developer install EPIC today?
*   **Locally (Developer Install)**: Yes. A developer can clone the repository, run `npm install`, and build the workspace. The Rust AST engine is compiled on demand:
    ```bash
    git clone https://github.com/akxh5/Solana-EPIC.git
    cd Solana-EPIC
    npm install
    npm run build
    cd packages/parser-v2 && cargo build --release
    ```
*   **Publicly (Registry-based Install)**: No. The packages are not yet published to the npm registry.

### 2. Can a brand-new developer run:
*   `epic audit .`
*   `epic check old_program new_program`

*   **Locally**: Yes, once the monorepo is built and the binary compiled locally, a developer can run commands using the compiled bundle:
    ```bash
    node packages/cli/dist/index.js audit .
    node packages/cli/dist/index.js check old_program new_program
    ```
*   **Publicly**: No, since the CLI package is not published, these commands are not accessible via global registry installations.

### 3. What currently prevents:
*   `npm install -g @epic/cli`
*   `npx @epic/cli audit .`

The following issues prevent registry-based installation:
1.  **Private Package Configuration**: In `package.json` files for `@epic/cli`, `@epic/parser`, `@epic/diff-engine`, and `@epic/github-action`, `"private": true` was set, which explicitly blocks publication. (Note: This has been updated to `"private": false` locally to prep for publication).
2.  **Placeholder Binaries**: The prebuilt packages for `@epic/cli-linux-x64` and `@epic/cli-win32-x64` contain only 41-byte placeholder text files rather than compiled executable binaries. A user installing on Linux or Windows will encounter runtime errors because the CLI wrapper will fail to load or execute the dummy file.

### 4. Which npm packages remain unpublished?
All packages in the monorepo remain unpublished to the public registry:
*   `@epic/cli` (CLI entry point)
*   `@epic/parser` (Config parsing and AST compilation bridge)
*   `@epic/diff-engine` (Layout upgrade analysis engine)
*   `@epic/github-action` (GitHub actions wrapper integration)
*   `@epic/cli-darwin-arm64` (Prebuilt binary package)
*   `@epic/cli-darwin-x64` (Prebuilt binary package)
*   `@epic/cli-linux-x64` (Prebuilt binary package - placeholder only)
*   `@epic/cli-win32-x64` (Prebuilt binary package - placeholder only)

### 5. Which binaries remain undistributed?
*   `parser-v2` for `linux-x64` (placeholder only)
*   `parser-v2.exe` for `win32-x64` (placeholder only)

The `darwin-arm64` and `darwin-x64` packages contain local compiled binaries, but they are not distributed on the registry.

### 6. Which release artifacts are missing?
*   Compiled executable parser binaries for Linux (`x86_64-unknown-linux-gnu`) and Windows (`x86_64-pc-windows-msvc`).
*   Automated cross-compilation pipeline scripts (or GitHub Action workflow) to build, test, and bundle these native binaries before registry publication.

### 7. What is the shortest path to public installation?
1.  **Remove `"private": true`** and configure public scope access (`"publishConfig": { "access": "public" }`) in all workspace package files (Completed).
2.  **Cross-Compile Native Binaries**: Set up a build server or run local toolchains to generate release versions of the Rust `parser-v2` binary for the remaining target architectures (Linux, Windows).
3.  **Bundle Binaries**: Place the compiled executables in their respective packages under `packages/cli-<platform>/bin/`.
4.  **Publish Platform Dependencies**: Execute `npm publish` on `@epic/cli-darwin-arm64`, `@epic/cli-darwin-x64`, `@epic/cli-linux-x64`, and `@epic/cli-win32-x64`.
5.  **Publish Core Packages**: Execute `npm publish` on `@epic/parser`, `@epic/diff-engine`, and finally `@epic/cli`.

---

## Technical Blockers & Estimated Effort

| Blocker | Affected Files | Target Actions | Est. Effort |
| :--- | :--- | :--- | :--- |
| **Private status flags** | `packages/{cli,parser,diff-engine,github-action}/package.json` | Set `"private": false` and add `"publishConfig"` | **Completed** |
| **Non-macOS Binary Assets** | `packages/cli-{linux-x64,win32-x64}/bin/` | Cross-compile Rust `parser-v2` binaries for linux and windows and copy to directories | 2 hours |
| **Publishing Pipeline** | Workspace root | Run `npm publish` in topological order (prebuilt binaries first, then libraries, then CLI) | 30 mins |

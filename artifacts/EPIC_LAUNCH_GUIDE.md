# EPIC Launch Guide

This guide is designed to be executed by a tired founder at 2 AM. Follow these steps sequentially to publish and verify the EPIC public release candidate.

---

## Step 1: Authentication

Ensure you are logged into your npm account with publishing permissions for the `@epic` scope.
```bash
npm login
```
Verify your login credentials:
```bash
npm whoami
```

---

## Step 2: Publication Order

Run these commands sequentially from the root of the monorepo. Scoped packages default to private, so you must pass `--access public` explicitly.

```bash
# 1. Publish native prebuilt binary targets
cd packages/cli-darwin-arm64 && npm publish --access public
cd ../cli-darwin-x64 && npm publish --access public
cd ../cli-linux-x64 && npm publish --access public
cd ../cli-win32-x64 && npm publish --access public

# 2. Publish shared libraries
cd ../parser && npm publish --access public
cd ../diff-engine && npm publish --access public

# 3. Publish CLI binary loader
cd ../cli && npm publish --access public

# 4. Publish GitHub Action integration
cd ../github-action && npm publish --access public
```

---

## Step 3: Fresh Install Verification

Open a fresh terminal window outside this repository and verify that npm downloads and resolves the CLI wrapper:
```bash
# Verify global installability
npm install -g @epic/cli

# Run rules listing to verify binary resolution
epic rules

# Verify explanation output
epic explain EPIC-SEC-001
```

---

## Step 4: Demo Verification

Test the published package against the demo fixtures:
```bash
# Run security audit on vulnerable demo
epic audit /path/to/Solana-EPIC/demo/security_vulnerable

# Run upgrade safety check on critical demo
epic check /path/to/Solana-EPIC/demo/upgrade_old /path/to/Solana-EPIC/demo/upgrade_new_critical
```

---

## Step 5: Rollback Plan

If a publish command fails halfway through the pipeline (e.g. registry error, network timeout, package version collision):

1.  **Do NOT try to re-publish the same version**: NPM strictly blocks republishing the same version string (`0.1.0-beta.1`).
2.  **Deprecate broken versions**: If a package was successfully published but has an error or is out of sync, deprecate it so users don't install it:
    ```bash
    npm deprecate @epic/cli@0.1.0-beta.1 "Critical publishing error. Please install latest."
    ```
3.  **Increment and Re-publish**:
    *   Increment the version string across all `package.json` files to `0.1.0-beta.2`.
    *   Run `npm run build` to compile target typescript folders.
    *   Restart the publish sequence from Step 2.

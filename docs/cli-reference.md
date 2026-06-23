# CLI Reference

EPIC CLI (`epic`) provides program analysis, upgrade safety validation, and security auditing for Solana programs.

---

## Commands

### 1. `epic audit`
Run security rules against a repository.
```bash
epic audit [path] [options]
```
*   **Arguments**:
    *   `[path]`: Directory or file path to scan (defaults to `.`).
*   **Options**:
    *   `-f, --format <format>`: Output format. Supported options: `text`, `json`, `sarif` (defaults to `text`).
    *   `-s, --strict`: If set, exits with code 1 if findings severity exceeds the configured threshold.
    *   `-c, --config <path>`: Path to a custom `epic.toml` configuration.
    *   `--ignore <rules>`: Comma-separated list of Rule IDs to ignore (e.g. `--ignore EPIC-SEC-001,EPIC-SEC-003`).

### 2. `epic check`
Compare two Solana program versions and validate upgrade compatibility.
```bash
epic check <old_path> <new_path> [options]
```
*   **Arguments**:
    *   `<old_path>`: Path to the old program version source directory.
    *   `<new_path>`: Path to the new program version source directory.
*   **Options**:
    *   `-c, --config <path>`: Path to `epic.toml` configuration.

### 3. `epic analyze`
Scan a Solana program workspace and report state account sizes.
```bash
epic analyze <path>
```
*   **Arguments**:
    *   `<path>`: Path to Anchor project, source directory, or file.

### 4. `epic rules`
List all security rules implemented in EPIC.
```bash
epic rules
```

### 5. `epic explain`
Print detailed explanations, threat models, and code examples for a specific rule.
```bash
epic explain <rule_id>
```
*   **Arguments**:
    *   `<rule_id>`: The rule identifier (e.g. `EPIC-SEC-004`).

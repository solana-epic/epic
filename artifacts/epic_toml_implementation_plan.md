# EPIC Configuration System: `epic.toml` Implementation Plan

This document provides the technical implementation plan for integrating `epic.toml` configuration parsing, schema validation, and override resolution into the EPIC TypeScript monorepo.

---

## 1. TypeScript Interfaces

We will define two layers of interfaces: the raw parsed TOML structure (representing the exact file layout) and the normalized, resolved configuration structure used internally by the diff and analysis engines.

```typescript
// packages/parser/src/config/types.ts

export type CompareMode = "ast" | "idl";
export type SeverityLevel = "SAFE" | "MINOR" | "MAJOR" | "CRITICAL";
export type OverrideAction = "allow" | "downgrade";

/**
 * Raw structure representing the exact layout of the epic.toml file on disk.
 */
export interface RawOverrideConfig {
  account: string;
  finding: string;
  field?: string;
  action: OverrideAction;
  severity?: SeverityLevel;
  note: string;
}

export interface RawProgramConfig {
  path: string;
  id: string;
  idl_path?: string;
  overrides?: RawOverrideConfig[];
}

export interface RawWorkspaceConfig {
  compare_mode?: CompareMode;
  fail_on_severity?: SeverityLevel;
  rpc_url?: string;
  exclude_paths?: string[];
  enforce_padding?: boolean;
}

export interface RawEpicConfig {
  workspace?: RawWorkspaceConfig;
  programs?: Record<string, RawProgramConfig>;
}

/**
 * Normalized structure resolved by the configuration engine.
 * Guarantees default values, resolves relative paths, and normalizes casings.
 */
export interface ResolvedOverride {
  account: string;      // Normalized to uppercase / matching casing rules
  finding: string;      // Standardized finding kind
  field?: string;       // Optional target field
  action: OverrideAction;
  severity?: SeverityLevel;
  note: string;
  used: boolean;        // Tracked at runtime to report stale overrides
}

export interface ResolvedProgram {
  name: string;
  absolutePath: string;
  programId: string;
  idlPath?: string;
  overrides: ResolvedOverride[];
}

export interface ResolvedEpicConfig {
  compareMode: CompareMode;
  failOnSeverity: SeverityLevel;
  rpcUrl?: string;
  excludePaths: string[];
  enforcePadding: boolean;
  programs: Map<string, ResolvedProgram>;
}
```

---

## 2. Folder Structure

The configuration module will be fully contained inside `@epic/parser` under a dedicated `config` namespace to avoid monorepo pollution and keep dependencies localized:

```
packages/parser/
├── src/
│   ├── config/
│   │   ├── index.ts          # Public entrypoint for config module
│   │   ├── types.ts          # TypeScript interfaces
│   │   ├── schema.ts         # Zod schemas for runtime validation
│   │   ├── loader.ts         # File discovery, TOML loading, and merging
│   │   └── validator.ts      # Custom guardrail checks (notes, banned overrides)
│   ├── index.ts              # Exports config namespace: export * as config from "./config/index.js";
│   └── ... (existing parser files)
└── test/                     # Node.js native test suite
    └── config.test.ts        # Unit tests covering loader, validation, and resolution
```

---

## 3. Required npm Dependencies

We will add the following npm packages to `@epic/parser` dependencies:

1.  **`smol-toml`** (v1.3.0+): Pure ESM, high-performance, specification-compliant TOML parser with zero external dependencies.
2.  **`zod`** (v3.23.0+): Standard typesafe validation library. Used to assert the structural schema of the parsed TOML.
3.  **`picomatch`** (v4.0.0+): Extremely fast and lightweight glob-matching library used to match files against the workspace `exclude_paths`.

```bash
# Execute within packages/parser directory
npm install smol-toml zod picomatch
```

---

## 4. Validation Architecture

To guarantee the integrity of the safety gates, configuration parsing runs through a three-stage validation pipeline before executing any comparisons.

```
[File on Disk] 
      │
      ▼
1. TOML Parse (smol-toml) ──► Fails on invalid syntax
      │
      ▼
2. Schema Verify (Zod)    ──► Fails on invalid types, missing keys, or empty paths
      │
      ▼
3. Security Audit (Custom) ──► Fails on wildcards, banned overrides, or short notes
      │
      ▼
[Resolved Config]
```

### Stage 1: TOML Syntax Ingestion
Reads the config file. Uses `smol-toml` to parse string content. Any syntax error immediately aborts execution with a detailed print of line and column info.

### Stage 2: Structural Validation (Zod Schema)
We define the Zod schema to validate shapes and compile structured error messages.

```typescript
// packages/parser/src/config/schema.ts
import { z } from "zod";

export const OverrideSchema = z.object({
  account: z.string().min(1, "Account name cannot be empty"),
  finding: z.string().min(1, "Finding kind cannot be empty"),
  field: z.string().optional(),
  action: z.enum(["allow", "downgrade"]),
  severity: z.enum(["SAFE", "MINOR", "MAJOR", "CRITICAL"]).optional(),
  note: z.string().min(10, "Override note must be at least 10 characters long to maintain audit logs")
}).refine(data => {
  if (data.action === "downgrade" && !data.severity) {
    return false;
  }
  return true;
}, {
  message: "Field 'severity' is required when action is 'downgrade'",
  path: ["severity"]
});

export const ProgramConfigSchema = z.object({
  path: z.string().min(1, "Program path must be defined"),
  id: z.string().min(1, "Program ID must be defined"),
  idl_path: z.string().optional(),
  overrides: z.array(OverrideSchema).default([])
});

export const EpicConfigSchema = z.object({
  workspace: z.object({
    compare_mode: z.enum(["ast", "idl"]).default("ast"),
    fail_on_severity: z.enum(["SAFE", "MINOR", "MAJOR", "CRITICAL"]).default("CRITICAL"),
    rpc_url: z.string().url("rpc_url must be a valid http/https endpoint").optional(),
    exclude_paths: z.array(z.string()).default([]),
    enforce_padding: z.boolean().default(false)
  }).default({}),
  programs: z.record(z.string(), ProgramConfigSchema).default({})
});
```

### Stage 3: Security Guardrails Validation
Custom checks running after Zod parsing:
1.  **Anti-Wildcard Check**: Rejects config if any `account`, `field`, or `finding` field is exactly `"*"` or contains wildcards to prevent developers from silencing entire structs.
2.  **Banned Override Protection (The Unbreakables)**:
    ```typescript
    const BANNED_FINDINGS = new Set(["FIELD_REMOVED", "FIELD_REORDERED"]);
    for (const [programName, program] of Object.entries(rawConfig.programs || {})) {
      for (const override of program.overrides || []) {
        if (BANNED_FINDINGS.has(override.finding.toUpperCase())) {
          throw new Error(
            `Security Violation: Overriding critical layout mutations (${override.finding}) is strictly forbidden. ` +
            `Offending account: ${override.account} in program: ${programName}`
          );
        }
      }
    }
    ```

---

## 5. Override Resolution Algorithm

When the comparison engine runs, it yields a series of findings. The resolution engine updates the severity of these findings based on the active config:

```typescript
// packages/diff-engine/src/resolve.ts
import type { DiffFinding, Severity } from "./compare.js";
import type { ResolvedEpicConfig } from "@epic/parser/config";

export interface ResolutionSummary {
  findings: DiffFinding[];
  appliedOverridesCount: number;
}

export function resolveFindingsWithConfig(
  programName: string,
  findings: DiffFinding[],
  config: ResolvedEpicConfig
): ResolutionSummary {
  const programConfig = config.programs.get(programName);
  if (!programConfig || programConfig.overrides.length === 0) {
    return { findings, appliedOverridesCount: 0 };
  }

  let appliedCount = 0;
  const updatedFindings = findings.map(finding => {
    // 1. Look for matching overrides
    const match = programConfig.overrides.find(override => {
      const accountMatch = override.account.toLowerCase() === finding.account.toLowerCase();
      const findingMatch = override.finding.toUpperCase() === finding.kind.toUpperCase();
      const fieldMatch = !override.field || (finding.field && override.field.toLowerCase() === finding.field.name.toLowerCase());
      return accountMatch && findingMatch && fieldMatch;
    });

    if (!match) {
      return finding; // No override active
    }

    // 2. Mark override as used
    match.used = true;
    appliedCount++;

    // 3. Resolve severity
    if (match.action === "allow") {
      return { ...finding, severity: "SAFE" as Severity };
    } else if (match.action === "downgrade") {
      return { ...finding, severity: (match.severity ?? "SAFE") as Severity };
    }

    return finding;
  });

  return { findings: updatedFindings, appliedOverridesCount: appliedCount };
}
```

---

## 6. CLI Integration Points

### A. Auto-discovery and Arguments
In `packages/cli/src/index.ts`, add the `--config <path>` option to `epic check` and `epic analyze`:
```typescript
program
  .command("check")
  .option("-c, --config <path>", "Path to epic.toml configuration file")
  .argument("<old_path>", "Path to the old program version source directory")
  .argument("<new_path>", "Path to the new program version source directory")
  .action(async (oldPath, newPath, options) => {
     const config = await loadConfig(options.config); // Finds config or returns default config
     // ...
  });
```

### B. Exit Codes
Adjust exit codes according to the `fail_on_severity` configuration:
```typescript
const severityOrder: SeverityLevel[] = ["SAFE", "MINOR", "MAJOR", "CRITICAL"];
const thresholdIndex = severityOrder.indexOf(config.failOnSeverity);
const maxFindingSeverityIndex = severityOrder.indexOf(report.severity);

if (maxFindingSeverityIndex >= thresholdIndex && maxFindingSeverityIndex !== -1) {
  console.error(`❌ EPIC Guard Blocked: Upgrade severity is ${report.severity} (threshold: ${config.failOnSeverity}).`);
  process.exit(1);
} else {
  console.log(`✅ EPIC Guard Approved Upgrade.`);
  process.exit(0);
}
```

---

## 7. GitHub Action Integration Points

### A. Git Diff Audit
The GitHub Action must detect if `epic.toml` has been modified:
1.  Run `git diff --name-only origin/main...HEAD` (or the target ref).
2.  If `epic.toml` or `**/epic.toml` is present in the list, set a flag: `const configModified = true;`
3.  Inject a prominent warn box into the PR comment markdown:
    ```markdown
    > [!WARNING]
    > **SECURITY GATE MODIFIED**
    > This Pull Request contains changes to `epic.toml`.
    > Review the layout rules and overrides carefully to ensure safety measures are not bypassed.
    ```

### B. Applied Overrides Table
Include an auditing section listing every applied override and its associated audit note:
```markdown
### 🔑 Applied Layout Overrides
| Account | Finding Type | Field | Action | Note |
| :--- | :--- | :--- | :--- | :--- |
| `Bank` | `PADDING_REPURPOSE` | `reserved` | `ALLOW` | Verified byte layouts align exactly with padding. |
```

### C. Stale Overrides Notification
Identify any registered overrides that were not flagged as `used` during the analysis:
```markdown
> [!NOTE]
> **Stale Configuration Detected**
> The following overrides in your `epic.toml` were not applied to any modified fields.
> Please clean up old configurations:
> *   `UserState` -> `FIELD_ADDED` (`max_margin_ratio`)
```

---

## 8. Unit Test Matrix

We will build the test suite using Node.js's native `node:test` framework under `packages/parser/test/config.test.ts`:

| Test Target | Scenario | Input | Expected Output |
| :--- | :--- | :--- | :--- |
| **Parser** | Valid File | Standard TOML | Resolved object structure populated with defaults |
| **Parser** | Malformed TOML | Missing brackets, invalid keys | Throws `TOMLParsingError` |
| **Validator**| Wildcard Blocking | `account = "*"` | Throws `ConfigValidationError` |
| **Validator**| Banned Overrides | `finding = "FIELD_REORDERED"` | Throws `SecurityViolationError` |
| **Validator**| Short Notes | `note = "Safe"` (4 chars) | Throws `ConfigValidationError` |
| **Validator**| Downgrade Missing | `action = "downgrade"`, no severity | Throws `ConfigValidationError` |
| **Resolver** | Apply Mute | Finding of `FIELD_ADDED` | Severity set to `SAFE`, `override.used = true` |
| **Resolver** | Match Specific | Override specifying field vs global struct | Applies the specific field rule |

---

## 9. Edge Cases

1.  **Casing Discrepancies**: 
    *   *Symptom*: TOML lists struct `bank`, while AST parses `Bank`.
    *   *Solution*: All comparisons between TOML overrides and AST/IDL findings must run normalized to lowercase: `override.account.toLowerCase() === finding.account.toLowerCase()`.
2.  **Monorepo Relative Paths**: 
    *   *Symptom*: Config is at `/workspace/epic.toml` and program defines `path = "./programs/drift"`, but CLI is executed inside `/workspace/packages/cli`.
    *   *Solution*: Resolve program paths relative to the directory containing `epic.toml` using `path.resolve(path.dirname(configFilePath), program.path)`.
3.  **Multiple Config Files**: 
    *   *Symptom*: Workspace root has `epic.toml` and program has `programs/drift/epic.toml`.
    *   *Solution*: Merge settings. Load root config, load local program config, and merge program config overrides into the root config registry, overwrite metadata if defined in both.
4.  **No Findings to Process**: 
    *   *Symptom*: Analysis is clean.
    *   *Solution*: Check all defined overrides and mark them as unused, warning the developer if they are stale.

---

## 10. Exact Coding Tasks in Execution Order

### Task 1: Initialize Dependencies
*   Add `smol-toml`, `zod`, and `picomatch` to `packages/parser/package.json`.
*   Run `npm install` from the monorepo root to link workspace packages and download packages.

### Task 2: Define Types & Zod Schemas
*   Create `packages/parser/src/config/types.ts` containing interface definitions.
*   Create `packages/parser/src/config/schema.ts` containing the structural Zod validator.

### Task 3: Implement Config Loader & Validator
*   Create `packages/parser/src/config/loader.ts` to locate `epic.toml` traversing upwards, parse it, and normalize output.
*   Create `packages/parser/src/config/validator.ts` containing custom wildcard, notes, and banned overrides validations.

### Task 4: Write Core Unit Tests
*   Create mock configuration fixtures.
*   Add tests in `packages/parser/test/config.test.ts` to check loader, validators, and edge cases. Make sure tests fail and pass on expected conditions.

### Task 5: Implement Resolution Logic in Diff Engine
*   Create `packages/diff-engine/src/resolve.ts` defining `resolveFindingsWithConfig`.
*   Connect `resolveFindingsWithConfig` inside `compareAccountLayouts` in `packages/diff-engine/src/compare.ts`.
*   Add unit tests in `packages/diff-engine/test/resolve.test.ts` to verify overrides modify findings correctly.

### Task 6: Integrate with CLI
*   Update `packages/cli/src/index.ts` to support the `--config` parameter.
*   Add config loading into the `check` and `analyze` command actions.
*   Update exit logic based on `fail_on_severity`.

### Task 7: Integrate with GitHub Action
*   Add the optional `config_path` input parameter in `action.yml`.
*   Implement git diff tracking for `epic.toml` changes.
*   Update the PR comment formatter to include the **Applied Overrides** table and **Stale Overrides** notification.

import path from "node:path";
import type { DiffFinding, Severity } from "./compare.js";
import type { config } from "@epic/parser";

const BANNED_FINDINGS = new Set(["FIELD_REMOVED", "FIELD_REORDERED"]);

/**
 * Attempts to match the compared directory path to a registered program config.
 */
export function findProgramName(dir: string, cfg: config.ResolvedEpicConfig): string | null {
  const resolvedDir = path.resolve(dir);
  for (const [name, program] of cfg.programs.entries()) {
    const progPath = path.resolve(program.absolutePath);
    if (resolvedDir === progPath || resolvedDir.startsWith(progPath + path.sep)) {
      return name;
    }
  }
  return null;
}

/**
 * Applies active configuration overrides to the generated diff findings.
 */
export function resolveFindingsWithConfig(
  programName: string,
  findings: DiffFinding[],
  cfg: config.ResolvedEpicConfig
): DiffFinding[] {
  const programConfig = cfg.programs.get(programName);
  if (!programConfig || programConfig.overrides.length === 0) {
    return findings;
  }

  return findings.map(finding => {
    // 1. Double check security: block overrides for banned critical types
    if (BANNED_FINDINGS.has(finding.kind.toUpperCase())) {
      return finding; // Ignore any overrides targeting reordering/removal
    }

    // 2. Look for specific field-level override first
    let match = programConfig.overrides.find((override: config.ResolvedOverride) => {
      const accountMatch = override.account.toLowerCase() === finding.account.toLowerCase();
      const findingMatch = override.finding.toUpperCase() === finding.kind.toUpperCase();
      const fieldMatch = finding.field && override.field && override.field.toLowerCase() === finding.field.name.toLowerCase();
      return !!(accountMatch && findingMatch && fieldMatch);
    });

    // 3. Fallback to global struct-level override
    if (!match) {
      match = programConfig.overrides.find((override: config.ResolvedOverride) => {
        const accountMatch = override.account.toLowerCase() === finding.account.toLowerCase();
        const findingMatch = override.finding.toUpperCase() === finding.kind.toUpperCase();
        const noFieldOverride = !override.field;
        return !!(accountMatch && findingMatch && noFieldOverride);
      });
    }

    if (!match) {
      return finding;
    }

    // Mark override as used
    match.used = true;

    // Apply override action
    if (match.action === "allow") {
      return { ...finding, severity: "SAFE" as Severity };
    } else if (match.action === "downgrade") {
      return { ...finding, severity: (match.severity || "SAFE") as Severity };
    }

    return finding;
  });
}

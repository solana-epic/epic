import type { DiffFinding, DiffReport } from "./compare.js";
import { createUpgradeIntelligenceItem } from "./intelligence.js";

const RULE = "═══════════════════════════════";

export function formatHumanReport(report: DiffReport): string {
  if (report.findings.length === 0) {
    return [
      RULE,
      "EPIC UPGRADE REPORT",
      RULE,
      "Severity: SAFE",
      "Finding:",
      "No structural account layout changes detected.",
      ""
    ].join("\n");
  }

  return report.findings.map(formatFinding).join("\n");
}

export function riskForFinding(finding: DiffFinding): string | null {
  return createUpgradeIntelligenceItem(finding).riskCategory;
}

export function recommendationForFinding(finding: DiffFinding): string {
  return createUpgradeIntelligenceItem(finding).recommendation;
}

function formatFinding(finding: DiffFinding): string {
  const intelligence = createUpgradeIntelligenceItem(finding);
  const lines = [
    RULE,
    "EPIC UPGRADE REPORT",
    RULE,
    `Program: ${finding.account}`,
    `Severity: ${finding.severity}`,
    "Finding:",
    findingTitle(finding)
  ];

  if (finding.kind === "FIELD_ADDED" && finding.field?.newType) {
    lines.push(`${finding.field.name}: ${finding.field.newType}`);
    lines.push("Account Size:");
    lines.push(`${finding.oldSize} -> ${finding.newSize} bytes`);
  }

  if (finding.kind === "FIELD_REMOVED" && finding.field?.oldType) {
    lines.push(`${finding.field.name}: ${finding.field.oldType}`);
  }

  if (finding.kind === "TYPE_CHANGED" && finding.field?.oldType && finding.field.newType) {
    lines.push(finding.field.name);
    lines.push(`${finding.field.oldType} -> ${finding.field.newType}`);
  }

  if (finding.kind === "SIZE_REDUCED") {
    lines.push(`Account Size Shrink:`);
    lines.push(`${finding.oldSize} -> ${finding.newSize} bytes`);
  }

  if (finding.kind === "DISCRIMINATOR_CHANGED" && finding.field?.oldType) {
    lines.push(`Instruction '${finding.field.name}' modified:`);
    lines.push(`Old Discriminator: ${finding.field.oldType}`);
    if (finding.field.newType) {
      lines.push(`New Discriminator: ${finding.field.newType}`);
    } else {
      lines.push(`New Discriminator: (deleted)`);
    }
  }

  lines.push("Risk Category:");
  lines.push(intelligence.riskCategory);
  lines.push("Affected Surface:");
  for (const surface of intelligence.affectedSurface) {
    lines.push(`- ${surface}`);
  }

  lines.push("Recommendation:");
  lines.push(intelligence.recommendation);
  lines.push("");

  return lines.join("\n");
}

function findingTitle(finding: DiffFinding): string {
  switch (finding.kind) {
    case "FIELD_ADDED":
      return "Field Added:";
    case "FIELD_REMOVED":
      return "Field Removed:";
    case "FIELD_REORDERED":
      return "Field Reordered";
    case "TYPE_CHANGED":
      return "Type Changed:";
    case "SIZE_REDUCED":
      return "Account Size Reduced:";
    case "DISCRIMINATOR_CHANGED":
      return "Program Discriminator Mismatch:";
  }
}

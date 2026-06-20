import type { DiffFinding, DiffFindingKind, DiffReport } from "./compare.js";

export type RiskCategory =
  | "Serialization Break"
  | "Account Expansion"
  | "Account Shrink"
  | "Field Reorder"
  | "Enum Expansion"
  | "Dynamic Type Introduction";

export type AffectedSurface = "Existing Accounts" | "Client SDKs" | "Indexers" | "IDLs";

export type UpgradeIntelligenceItem = {
  account: string;
  field?: string;
  change: string;
  findingKind: DiffFindingKind;
  severity: DiffFinding["severity"];
  riskCategory: RiskCategory;
  affectedSurface: AffectedSurface[];
  recommendation: string;
};

export type UpgradeIntelligenceReport = {
  severity: DiffReport["severity"];
  items: UpgradeIntelligenceItem[];
};

export function createUpgradeIntelligence(report: DiffReport): UpgradeIntelligenceReport {
  return {
    severity: report.severity,
    items: report.findings.map(createUpgradeIntelligenceItem)
  };
}

export function createUpgradeIntelligenceItem(finding: DiffFinding): UpgradeIntelligenceItem {
  const riskCategory = riskCategoryForFinding(finding);

  return {
    account: finding.account,
    field: finding.field?.name,
    change: changeDescription(finding),
    findingKind: finding.kind,
    severity: finding.severity,
    riskCategory,
    affectedSurface: affectedSurfaceForCategory(riskCategory),
    recommendation: recommendationForCategory(riskCategory)
  };
}

function riskCategoryForFinding(finding: DiffFinding): RiskCategory {
  switch (finding.kind) {
    case "FIELD_ADDED":
      return isDynamicType(finding.field?.newType) ? "Dynamic Type Introduction" : "Account Expansion";
    case "FIELD_REMOVED":
      return finding.newSize < finding.oldSize ? "Account Shrink" : "Serialization Break";
    case "FIELD_REORDERED":
      return "Field Reorder";
    case "TYPE_CHANGED":
      return isDynamicType(finding.field?.newType) ? "Dynamic Type Introduction" : "Serialization Break";
  }
}

function affectedSurfaceForCategory(category: RiskCategory): AffectedSurface[] {
  switch (category) {
    case "Account Expansion":
      return ["Existing Accounts", "IDLs", "Client SDKs"];
    case "Account Shrink":
    case "Serialization Break":
    case "Field Reorder":
      return ["Existing Accounts", "Client SDKs", "Indexers", "IDLs"];
    case "Enum Expansion":
      return ["Client SDKs", "Indexers", "IDLs"];
    case "Dynamic Type Introduction":
      return ["Existing Accounts", "Client SDKs", "Indexers", "IDLs"];
  }
}

function recommendationForCategory(category: RiskCategory): string {
  switch (category) {
    case "Account Expansion":
      return "Add an explicit realloc path and rent top-up before writing the new field.";
    case "Account Shrink":
      return "Create a migration instruction before upgrade and avoid shrinking persisted accounts in place.";
    case "Serialization Break":
      return "Create migration instruction before upgrade.";
    case "Field Reorder":
      return "Do not reorder persisted fields; append new fields at the end or migrate into a new account layout.";
    case "Enum Expansion":
      return "Regenerate IDLs and client SDKs, then verify indexers handle the new enum variant.";
    case "Dynamic Type Introduction":
      return "Avoid introducing dynamic persisted fields without bounded sizing and an explicit migration plan.";
  }
}

function changeDescription(finding: DiffFinding): string {
  switch (finding.kind) {
    case "FIELD_ADDED":
      return `${finding.field?.name ?? "field"} added as ${finding.field?.newType ?? "unknown"}`;
    case "FIELD_REMOVED":
      return `${finding.field?.name ?? "field"} removed`;
    case "FIELD_REORDERED":
      return "Persisted field order changed";
    case "TYPE_CHANGED":
      return `${finding.field?.oldType ?? "unknown"} -> ${finding.field?.newType ?? "unknown"}`;
  }
}

function isDynamicType(type: string | undefined): boolean {
  if (!type) {
    return false;
  }

  return /^(Vec|String|HashMap|HashSet|BTreeMap|BTreeSet)\b/.test(type);
}

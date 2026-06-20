import type { DiffFinding, Severity } from "./compare.js";

const SEVERITY_ORDER: Severity[] = ["SAFE", "MINOR", "MAJOR", "CRITICAL"];

export function classifyFindings(findings: DiffFinding[]): DiffFinding[] {
  return findings.map((finding) => ({
    ...finding,
    severity: severityForFinding(finding)
  }));
}

export function highestSeverity(severities: Severity[]): Severity {
  if (severities.length === 0) {
    return "SAFE";
  }

  return severities.reduce((highest, severity) =>
    SEVERITY_ORDER.indexOf(severity) > SEVERITY_ORDER.indexOf(highest) ? severity : highest
  );
}

function severityForFinding(finding: DiffFinding): Severity {
  switch (finding.kind) {
    case "FIELD_ADDED":
      return finding.severity === "CRITICAL" ? "CRITICAL" : "MAJOR";
    case "FIELD_REMOVED":
    case "FIELD_REORDERED":
    case "TYPE_CHANGED":
      return "CRITICAL";
  }
}

import type { DiffReport } from "@epic/diff-engine";
import type { config } from "@epic/parser";
import { createUpgradeIntelligenceItem } from "@epic/diff-engine";

export function generateCompactMarkdownReport(
  report: DiffReport,
  cfg: config.ResolvedEpicConfig,
  configChanged: boolean
): string {
  const lines: string[] = [];

  // Determine Overall Status
  const failSeverity = cfg.failOnSeverity;
  const severityOrder = ["SAFE", "MINOR", "MAJOR", "CRITICAL"];
  const thresholdIndex = severityOrder.indexOf(failSeverity);
  const reportSeverityIndex = severityOrder.indexOf(report.severity);
  const blocked = thresholdIndex !== -1 && reportSeverityIndex !== -1 && reportSeverityIndex >= thresholdIndex;

  const hasOverrides = report.findings.some(f => {
    const original = f.kind === "FIELD_ADDED" ? "MAJOR" : "CRITICAL";
    return f.severity !== original;
  });

  // 1. Status Banner
  if (blocked) {
    lines.push("## 🔴 EPIC Guard: UPGRADE BLOCKED");
    lines.push("");
    lines.push(`Upgrade checks failed because layout changes exceed the **${failSeverity}** threshold.`);
  } else if (hasOverrides) {
    lines.push("## 🟡 EPIC Guard: APPROVED WITH OVERRIDES");
    lines.push("");
    lines.push("Upgrade checks passed with muted warnings. Custom overrides are active in `epic.toml`.");
  } else {
    lines.push("## 🟢 EPIC Guard: APPROVED");
    lines.push("");
    lines.push("Upgrade checks approved. No layout compatibility risks detected.");
  }
  lines.push("");

  // 2. Config Change Warning
  if (configChanged) {
    lines.push("> [!WARNING]");
    lines.push("> **UPGRADE CONFIGURATION GATE MODIFIED**");
    lines.push("> This Pull Request contains changes to `epic.toml` configuration rules.");
    lines.push("> Signers must audit the modifications below to ensure safety limits are not bypassed.");
    lines.push("");
  }

  // 3. Summary Table
  lines.push("### 📊 Upgrade Summary");
  lines.push("");
  lines.push("| Program | Account | Finding | Final Severity | Overridden? |");
  lines.push("| :--- | :--- | :--- | :--- | :--- |");
  
  if (report.findings.length === 0) {
    lines.push("| *N/A* | *All Accounts* | *No structural changes* | `SAFE` | No |");
  } else {
    for (const f of report.findings) {
      const original = f.kind === "FIELD_ADDED" ? "MAJOR" : "CRITICAL";
      const isOverridden = f.severity !== original;
      lines.push(`| \`marginfi\` | \`${f.account}\` | \`${f.kind}\` | \`${f.severity}\` | ${isOverridden ? "✅ Yes" : "No"} |`);
    }
  }
  lines.push("");

  // 4. Detailed Findings
  if (report.findings.length > 0) {
    lines.push("### 🔍 Layout Findings");
    lines.push("");
    for (const f of report.findings) {
      const intel = createUpgradeIntelligenceItem(f);
      lines.push(`#### Struct \`${f.account}\` — **${f.kind}**`);
      lines.push("");
      lines.push(`*   **Finding Type**: ${f.kind}`);
      if (f.field) {
        lines.push(`*   **Field**: \`${f.field.name}\` (${f.field.oldType || "new"} ──► ${f.field.newType || "removed"})`);
      }
      lines.push(`*   **Size Impact**: \`${f.oldSize}B\` ──► \`${f.newSize}B\` (${f.newSize - f.oldSize >= 0 ? "+" : ""}${f.newSize - f.oldSize} bytes)`);
      lines.push(`*   **Risk Class**: ${intel.riskCategory}`);
      lines.push(`*   **Severity**: \`${f.severity}\``);
      lines.push("");
    }
  }

  // 5. Applied Overrides Section
  const appliedOverrides: Array<{ account: string; finding: string; field?: string; shift: string; note: string }> = [];
  for (const f of report.findings) {
    const original = f.kind === "FIELD_ADDED" ? "MAJOR" : "CRITICAL";
    if (f.severity !== original) {
      // Find override note
      let note = "No note provided.";
      for (const [name, program] of cfg.programs.entries()) {
        const match = program.overrides.find(o => {
          const accountMatch = o.account.toLowerCase() === f.account.toLowerCase();
          const findingMatch = o.finding.toUpperCase() === f.kind.toUpperCase();
          const fieldMatch = f.field && o.field && o.field.toLowerCase() === f.field.name.toLowerCase();
          return accountMatch && findingMatch && (fieldMatch || !o.field);
        });
        if (match) {
          note = match.note;
          break;
        }
      }
      appliedOverrides.push({
        account: f.account,
        finding: f.kind,
        field: f.field?.name,
        shift: `\`${original}\` ──► \`${f.severity}\``,
        note
      });
    }
  }

  if (appliedOverrides.length > 0) {
    lines.push("### 🔑 Applied Layout Overrides");
    lines.push("");
    lines.push("| Struct | Finding | Field | Severity Shift | Note / Safety Justification |");
    lines.push("| :--- | :--- | :--- | :--- | :--- |");
    for (const o of appliedOverrides) {
      lines.push(`| \`${o.account}\` | \`${o.finding}\` | \`${o.field || "global"}\` | ${o.shift} | ${o.note} |`);
    }
    lines.push("");
  }

  // 6. Recommended Actions
  lines.push("### 💡 Recommended Actions");
  lines.push("");
  if (report.findings.length === 0) {
    lines.push("*   No action required. Layout upgrades are safe to proceed.");
  } else {
    const uniqueRecommendations = new Set<string>();
    for (const f of report.findings) {
      const intel = createUpgradeIntelligenceItem(f);
      uniqueRecommendations.add(intel.recommendation);
    }
    for (const rec of uniqueRecommendations) {
      lines.push(`*   ${rec}`);
    }
  }
  lines.push("");

  return lines.join("\n");
}

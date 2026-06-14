#!/usr/bin/env node
import { Command } from "commander";
import { analyzeAnchorProject, compareAnchorProjects, type AccountDiff } from "@epic/parser";

const program = new Command();

program
  .name("epic")
  .description("EPIC CLI foundation for analyzing Anchor projects.")
  .version("0.1.0");

program
  .command("analyze")
  .description("Analyze an Anchor project and report #[account] struct sizes.")
  .argument("<path>", "Path to an Anchor project, Rust source directory, or Rust file")
  .action(async (targetPath: string) => {
    try {
      const result = await analyzeAnchorProject(targetPath);

      if (result.accounts.length === 0) {
        console.log("No #[account] structs found.");
        return;
      }

      for (const account of result.accounts) {
        console.log(`${account.name}: ${account.byteSize} bytes`);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      console.error(`epic analyze failed: ${message}`);
      process.exitCode = 1;
    }
  });

program
  .command("check")
  .description("Compare two Anchor project versions and report upgrade readiness.")
  .argument("<old_path>", "Path to the old Anchor project, Rust source directory, or Rust file")
  .argument("<new_path>", "Path to the new Anchor project, Rust source directory, or Rust file")
  .action(async (oldPath: string, newPath: string) => {
    try {
      const report = await compareAnchorProjects(oldPath, newPath);
      console.log(formatUpgradeReadinessReport(report.accountDiffs, report.accountsChanged, report.overallRisk));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      console.error(`epic check failed: ${message}`);
      process.exitCode = 1;
    }
  });

await program.parseAsync(process.argv);

function formatUpgradeReadinessReport(
  accountDiffs: AccountDiff[],
  accountsChanged: number,
  overallRisk: string
): string {
  const lines: string[] = [
    "====================================",
    "EPIC Upgrade Readiness Report",
    "",
    `Accounts Changed: ${accountsChanged}`
  ];

  for (const diff of accountDiffs) {
    lines.push("", diff.name, "");

    if (diff.oldSize !== null) {
      lines.push(`Old Size: ${diff.oldSize}`);
    }

    if (diff.newSize !== null) {
      lines.push(`New Size: ${diff.newSize}`);
    }

    if (diff.addedFields.length > 0) {
      lines.push("", "Added:");
      for (const field of diff.addedFields) {
        lines.push(`* ${field.name} (${field.type})`);
      }
    }

    if (diff.removedFields.length > 0) {
      lines.push("", "Removed:");
      for (const field of diff.removedFields) {
        lines.push(`* ${field.name} (${field.type})`);
      }
    }

    if (diff.typeChangedFields.length > 0) {
      lines.push("", "Type Changes:");
      for (const field of diff.typeChangedFields) {
        lines.push(`* ${field.name}: ${field.oldType} -> ${field.newType}`);
      }
    }

    lines.push(
      "",
      `Migration Required: ${diff.migrationRequired ? "YES" : "NO"}`,
      "",
      `Risk Level: ${diff.riskLevel}`,
      "",
      "Recommended Actions:"
    );

    for (const recommendation of diff.recommendations) {
      lines.push(`* ${recommendation}`);
    }
  }

  if (accountDiffs.length === 0) {
    lines.push("", "No account layout changes detected.");
  }

  lines.push("", "====================================", `Overall Risk: ${overallRisk}`);
  return lines.join("\n");
}

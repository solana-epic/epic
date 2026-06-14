#!/usr/bin/env node
import { Command } from "commander";
import {
  analyzeAnchorProject,
  compareAnchorProjects,
  simulateUpgrade,
  toMachineReadableSimulation,
  toMachineReadableReport,
  type AccountDiff,
  type UpgradeSimulation
} from "@epic/parser";

const program = new Command();

program
  .name("epic")
  .description("EPIC CLI foundation for analyzing Anchor projects.")
  .version("0.4.0");

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
  .option("--json", "Print machine-readable JSON for CI and integrations")
  .action(async (oldPath: string, newPath: string, options: { json?: boolean }) => {
    try {
      const report = await compareAnchorProjects(oldPath, newPath);

      if (options.json) {
        console.log(JSON.stringify(toMachineReadableReport(report), null, 2));
        return;
      }

      console.log(formatUpgradeReadinessReport(report.accountDiffs, report.accountsChanged, report.overallRisk));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      console.error(`epic check failed: ${message}`);
      process.exitCode = 1;
    }
  });

program
  .command("simulate")
  .description("Simulate a program upgrade using EPIC's static upgrade engine.")
  .argument("<old_path>", "Path to the old Anchor project, Rust source directory, or Rust file")
  .argument("<new_path>", "Path to the new Anchor project, Rust source directory, or Rust file")
  .option("--json", "Print machine-readable JSON for CI and future Bankrun integrations")
  .action(async (oldPath: string, newPath: string, options: { json?: boolean }) => {
    try {
      const simulation = await simulateUpgrade(oldPath, newPath);

      if (options.json) {
        console.log(JSON.stringify(toMachineReadableSimulation(simulation), null, 2));
        return;
      }

      console.log(formatUpgradeSimulation(simulation));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      console.error(`epic simulate failed: ${message}`);
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

    if (diff.sizeDelta !== null) {
      lines.push(`Size Delta: ${formatSignedBytes(diff.sizeDelta)}`);
    }

    lines.push(`Additional Bytes Required: ${diff.additionalBytesRequired}`);
    lines.push(`ABI Fingerprint Changed: ${diff.abiFingerprintChanged ? "YES" : "NO"}`);

    if (diff.fieldReordered) {
      lines.push("Reason: Field Reorder Detected");
    }

    if (diff.reasons.length > 0) {
      lines.push("", "Reasons:");
      for (const reason of diff.reasons) {
        lines.push(`* ${reason}`);
      }
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

    if (diff.layoutWarnings.length > 0) {
      lines.push("", "WARNING:", "Dynamic size detected.", "", "Static realloc analysis may be inaccurate.");
      for (const warning of diff.layoutWarnings) {
        lines.push(`* ${warning.account}.${warning.field} (${warning.type})`);
      }
    }

    lines.push(
      "",
      `Migration Required: ${diff.migrationRequired ? "YES" : "NO"}`,
      "",
      `Risk Level: ${diff.riskLevel}`,
      `Migration Complexity: ${diff.complexity}`,
      "",
      `Realloc Required: ${diff.realloc.required ? "YES" : "NO"}`
    );

    if (diff.realloc.suggestedAction) {
      lines.push(
        "",
        "Suggested Action:",
        "",
        diff.realloc.suggestedAction
      );
    }

    lines.push(
      "",
      `Rent Impact: ${diff.rentImpact.status}`,
      `Estimated Additional Bytes: ${diff.rentImpact.estimatedAdditionalBytes}`,
      `Future Hook: ${diff.rentImpact.futureHook}`,
      "",
      "Recommended Upgrade Plan"
    );

    diff.upgradePlan.forEach((step, index) => {
      lines.push(`${index + 1}. ${step}`);
    });

    if (diff.recommendations.length > 0) {
      lines.push("", "Recommended Actions:");
      for (const recommendation of diff.recommendations) {
        lines.push(`* ${recommendation}`);
      }
    }
  }

  if (accountDiffs.length === 0) {
    lines.push("", "No account layout changes detected.");
  }

  lines.push("", "====================================", `Overall Risk: ${overallRisk}`);
  return lines.join("\n");
}

function formatSignedBytes(delta: number): string {
  if (delta > 0) {
    return `+${delta}`;
  }

  return String(delta);
}

function formatUpgradeSimulation(simulation: UpgradeSimulation): string {
  const lines: string[] = [
    "====================================",
    "EPIC Upgrade Simulation",
    "",
    `Simulation Mode: ${simulation.mode}`,
    `Simulation Adapter: ${simulation.adapter}`,
    `Affected Accounts: ${simulation.affectedAccounts.length}`,
    `Risk Level: ${simulation.riskLevel}`,
    `Risk Score: ${simulation.riskScore}`,
    "",
    "Estimated Rent Increase:",
    `Additional Bytes: ${simulation.estimatedRentIncrease.additionalBytes}`,
    `Exact Lamports: ${simulation.estimatedRentIncrease.exactLamports ?? "Unavailable"}`,
    `Calculation Mode: ${simulation.estimatedRentIncrease.calculationMode}`,
    `Future Hook: ${simulation.estimatedRentIncrease.futureHook}`,
    "",
    "Affected Accounts"
  ];

  if (simulation.affectedAccounts.length === 0) {
    lines.push("No account layout changes detected.");
  }

  for (const account of simulation.affectedAccounts) {
    lines.push(
      "",
      account.name,
      `Status: ${account.status}`,
      `Old Size: ${account.oldSize ?? "N/A"}`,
      `New Size: ${account.newSize ?? "N/A"}`,
      `Size Delta: ${account.sizeDelta === null ? "N/A" : formatSignedBytes(account.sizeDelta)}`,
      `Migration Required: ${account.migrationRequired ? "YES" : "NO"}`,
      `Complexity: ${account.complexity}`,
      `Risk: ${account.riskLevel}`
    );
  }

  lines.push("", "Realloc Requirements");

  const reallocRequirements = simulation.reallocRequirements.filter((requirement) => requirement.required);

  if (reallocRequirements.length === 0) {
    lines.push("No realloc required.");
  }

  for (const requirement of reallocRequirements) {
    lines.push(
      "",
      requirement.account,
      `Required: YES`,
      `Old Size: ${requirement.oldSize ?? "N/A"}`,
      `New Size: ${requirement.newSize ?? "N/A"}`,
      `Additional Bytes Required: ${requirement.additionalBytesRequired}`
    );

    if (requirement.suggestedAction) {
      lines.push("", "Suggested Action:", "", requirement.suggestedAction);
    }
  }

  lines.push("", "Migration Plan");

  if (simulation.migrationPlan.length === 0) {
    lines.push("No migration steps required.");
  } else {
    simulation.migrationPlan.forEach((step, index) => {
      lines.push(`${index + 1}. ${step}`);
    });
  }

  lines.push(
    "",
    "Bankrun Integration",
    `Ready: ${simulation.bankrunReady ? "YES" : "NO"}`,
    `Hook: ${simulation.bankrunHook}`,
    "",
    "===================================="
  );

  return lines.join("\n");
}

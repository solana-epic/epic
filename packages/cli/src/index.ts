#!/usr/bin/env node
import { Command } from "commander";
import { compareAnchorPrograms, formatHumanReport } from "@epic/diff-engine";
import { config } from "@epic/parser";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import fs from "node:fs";

const program = new Command();

program
  .name("epic")
  .description("EPIC CLI for Solana Upgrade Intelligence (powered by parser-v2 Rust AST engine).")
  .version("0.4.0");

import { resolveParserBinary } from "./loader.js";

function findRustBinary(): string {
  try {
    return resolveParserBinary();
  } catch (err: any) {
    console.error(err.message);
    process.exit(1);
  }
}

program
  .command("analyze")
  .description("Analyze a Solana program workspace and report state account sizes.")
  .argument("<path>", "Path to an Anchor project, Rust source directory, or Rust file")
  .action((targetPath: string) => {
    try {
      const binary = findRustBinary();
      const resolvedPath = path.resolve(targetPath);
      
      const result = spawnSync(binary, [resolvedPath], { encoding: "utf-8" });
      
      if (result.error) {
        throw new Error(`Failed to execute parser-v2 binary: ${result.error.message}`);
      }
      
      if (result.status !== 0) {
        console.error(result.stderr || `Execution failed with status code ${result.status}`);
        process.exit(result.status ?? 1);
      }

      const report = JSON.parse(result.stdout.trim());
      
      console.log(`\n🔍 Analyzing Solana Program Workspace: ${targetPath}`);
      console.log(`Found ${report.structs_found} structs, ${report.enums_found} enums, ${report.aliases_found} aliases.\n`);
      
      if (!report.accounts || report.accounts.length === 0) {
        console.log("No state accounts (#[account] structures) found.");
        return;
      }

      console.log("STATE ACCOUNTS:");
      for (const account of report.accounts) {
        const layoutType = account.dynamic ? "Dynamic" : "Static";
        const prefix = account.dynamic ? "⚠️" : "├──";
        console.log(`${prefix} ${account.account} (${account.size} bytes) [${account.namespace}] [${layoutType}]`);
        if (account.dynamic) {
          console.log(`   └─ Warning: Dynamic size detected. Static layout realloc checks may be inaccurate.`);
        }
      }
      console.log("");
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      console.error(`epic analyze failed: ${message}`);
      process.exit(1);
    }
  });

program
  .command("check")
  .description("Compare two Solana program workspace versions and report upgrade readiness.")
  .option("-c, --config <path>", "Path to epic.toml configuration file")
  .argument("<old_path>", "Path to the old program version source directory")
  .argument("<new_path>", "Path to the new program version source directory")
  .action(async (oldPath: string, newPath: string, options: { config?: string }) => {
    try {
      const resolvedOldPath = path.resolve(oldPath);
      const resolvedNewPath = path.resolve(newPath);

      let epicConfig: config.ResolvedEpicConfig;
      try {
        epicConfig = config.loadEpicConfig(options.config);
      } catch (err: any) {
        console.error(`epic.toml validation error: ${err.message}`);
        process.exit(1);
      }

      const report = await compareAnchorPrograms(resolvedOldPath, resolvedNewPath, epicConfig);

      console.log(formatHumanReport(report));

      const severityOrder = ["SAFE", "MINOR", "MAJOR", "CRITICAL"];
      const thresholdIndex = severityOrder.indexOf(epicConfig.failOnSeverity);
      const reportSeverityIndex = severityOrder.indexOf(report.severity);

      if (thresholdIndex !== -1 && reportSeverityIndex !== -1 && reportSeverityIndex >= thresholdIndex) {
        console.error(`❌ EPIC Guard Blocked: Upgrade severity is ${report.severity} (threshold: ${epicConfig.failOnSeverity}).`);
        process.exit(1);
      } else {
        console.log(`✅ EPIC Guard Approved Upgrade.`);
        process.exit(0);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      console.error(`epic check failed: ${message}`);
      process.exit(1);
    }
  });

program.parse(process.argv);

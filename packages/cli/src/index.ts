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

function getSeverityLevel(sev: string): number {
  const s = sev.toUpperCase();
  if (s === "WARNING" || s === "WARN" || s === "SAFE" || s === "MINOR") return 0;
  if (s === "MEDIUM" || s === "MAJOR") return 1;
  if (s === "HIGH") return 2;
  if (s === "CRITICAL") return 3;
  return 3;
}

function generateSarif(findings: any[]): any {
  const rulesMap = new Map<string, any>();
  
  rulesMap.set("EPIC-SEC-001", {
    id: "EPIC-SEC-001",
    shortDescription: {
      text: "Owner Validation"
    },
    fullDescription: {
      text: "Unchecked mutable account write without dominating owner validation."
    },
    helpUri: "https://github.com/akxh5/Solana-EPIC/blob/main/docs/rules/EPIC-SEC-001.md",
    properties: {
      category: "Security",
      precision: "high"
    }
  });

  const results = findings.map((f) => {
    let level = "warning";
    const sev = f.severity.toLowerCase();
    if (sev === "critical" || sev === "high") {
      level = "error";
    } else if (sev === "medium") {
      level = "warning";
    } else if (sev === "warning" || sev === "low") {
      level = "note";
    }

    const relFile = path.relative(process.cwd(), f.location.file);

    return {
      ruleId: f.rule_id,
      ruleIndex: 0,
      level,
      message: {
        text: f.message
      },
      locations: [
        {
          physicalLocation: {
            artifactLocation: {
              uri: relFile,
              uriBaseId: "%SRCROOT%"
            },
            region: {
              startLine: f.location.line,
              startColumn: f.location.column || 1
            }
          }
        }
      ]
    };
  });

  const rules = Array.from(rulesMap.values());
  for (const f of findings) {
    if (!rulesMap.has(f.rule_id)) {
      const genericRule = {
        id: f.rule_id,
        shortDescription: {
          text: f.rule_id
        },
        fullDescription: {
          text: f.message
        }
      };
      rulesMap.set(f.rule_id, genericRule);
      rules.push(genericRule);
    }
  }

  return {
    $schema: "https://schemastore.azurewebsites.net/schemas/json/sarif-2.1.0-rtm.5.json",
    version: "2.1.0",
    runs: [
      {
        tool: {
          driver: {
            name: "EPIC",
            informationUri: "https://github.com/akxh5/Solana-EPIC",
            version: "0.4.0",
            rules
          }
        },
        results
      }
    ]
  };
}

program
  .command("audit")
  .description("Run security rules against the repository.")
  .argument("[path]", "Path to search and audit", ".")
  .option("-f, --format <format>", "Output format: text, json, sarif", "text")
  .option("-s, --strict", "Exit code 1 if findings severity >= threshold", false)
  .option("-c, --config <path>", "Path to epic.toml configuration file")
  .option("--ignore <rules>", "Rule IDs to ignore (comma-separated)", (val) => val.split(",").map(r => r.trim()))
  .action(async (targetPath: string, options: { format: string, strict: boolean, config?: string, ignore?: string[] }) => {
    try {
      const binary = findRustBinary();
      const resolvedPath = path.resolve(targetPath);
      
      const result = spawnSync(binary, ["audit", resolvedPath], { encoding: "utf-8" });
      
      if (result.error) {
        throw new Error(`Failed to execute parser-v2 binary: ${result.error.message}`);
      }
      
      if (result.status !== 0) {
        console.error(result.stderr || `Execution failed with status code ${result.status}`);
        process.exit(result.status ?? 1);
      }

      const findings = JSON.parse(result.stdout.trim());

      let epicConfig: config.ResolvedEpicConfig;
      try {
        epicConfig = config.loadEpicConfig(options.config);
      } catch (err) {
        epicConfig = config.getDefaultConfig();
      }

      const ignoredRules = new Set<string>();
      if (epicConfig.ignore) {
        for (const r of epicConfig.ignore) {
          ignoredRules.add(r.trim());
        }
      }
      if (options.ignore) {
        const cliIgnores = Array.isArray(options.ignore) ? options.ignore : [options.ignore];
        for (const r of cliIgnores) {
          ignoredRules.add(r.trim());
        }
      }

      const activeFindings = findings.filter((f: any) => !ignoredRules.has(f.rule_id));

      if (options.format === "text") {
        if (activeFindings.length === 0) {
          console.log("No security findings found.");
        } else {
          for (const finding of activeFindings) {
            const sevUpper = finding.severity.toUpperCase();
            const relPath = path.relative(process.cwd(), finding.location.file);
            console.log(`${sevUpper} ${finding.rule_id}`);
            console.log(`${relPath}:${finding.location.line}:${finding.location.column}`);
            console.log(`${finding.message}\n`);
          }
        }
      } else if (options.format === "json") {
        console.log(JSON.stringify(activeFindings, null, 2));
      } else if (options.format === "sarif") {
        const sarif = generateSarif(activeFindings);
        const sarifString = JSON.stringify(sarif, null, 2);
        fs.writeFileSync("sarif.json", sarifString, "utf8");
        console.log(sarifString);
      }

      if (options.strict) {
        const threshold = epicConfig.failOnSeverity || "CRITICAL";
        const thresholdVal = getSeverityLevel(threshold);
        
        let hasFailingFinding = false;
        for (const finding of activeFindings) {
          const sevVal = getSeverityLevel(finding.severity);
          if (sevVal >= thresholdVal) {
            hasFailingFinding = true;
            break;
          }
        }
        
        if (hasFailingFinding) {
          process.exit(1);
        } else {
          process.exit(0);
        }
      } else {
        process.exit(0);
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      console.error(`epic audit failed: ${message}`);
      process.exit(1);
    }
  });

program
  .command("rules")
  .description("List all available security rules.")
  .action(() => {
    console.log("EPIC-SEC-001");
    console.log("Owner Validation");
    console.log("Critical");
    console.log("Implemented");
  });

program
  .command("explain")
  .description("Explain a security rule in detail.")
  .argument("<rule_id>", "Rule ID to explain")
  .action(async (ruleId: string) => {
    const normRuleId = ruleId.trim().toUpperCase();
    if (normRuleId === "EPIC-SEC-001") {
      let content = "";
      try {
        const docPaths = [
          path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../../docs/rules/EPIC-SEC-001.md"),
          path.resolve(process.cwd(), "docs/rules/EPIC-SEC-001.md")
        ];
        for (const p of docPaths) {
          if (fs.existsSync(p)) {
            content = fs.readFileSync(p, "utf8");
            break;
          }
        }
      } catch (err) {
        // ignore error
      }
      
      if (content) {
        console.log(content);
      } else {
        console.log(`# EPIC-SEC-001: Owner Validation

## Description
Tracks mutable account write operations to ensure they are protected by an ownership check (\`account.owner == program_id\`) that dominates the write path.

## Threat Model
In Solana, any account can be passed to an instruction. If a program writes data to a mutable account without verifying that the account is owned by the program itself, an attacker can pass a forged account with malicious data.

## Vulnerable Example
\`\`\`rust
pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    let mut vault_data = vault.try_borrow_mut_data()?;
    vault_data[0] = 9;
    Ok(())
}
\`\`\`

## Safe Example
\`\`\`rust
#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub vault: Account<'info, VaultState>,
}
\`\`\`

## Historical Exploit References
* Cashio App ($52M, March 2022)`);
      }
    } else {
      console.log(`Rule ${ruleId} not found.`);
      process.exit(1);
    }
  });

program.parse(process.argv);

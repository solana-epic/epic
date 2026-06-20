import * as core from "@actions/core";
import { compareAnchorPrograms } from "@epic/diff-engine";
import { config } from "@epic/parser";
import { upsertPRComment, checkIfConfigChanged } from "./github.js";
import { generateCompactMarkdownReport } from "./report.js";

async function run(): Promise<void> {
  try {
    const githubToken = core.getInput("github_token", { required: true });
    const oldPath = core.getInput("old_path", { required: true });
    const newPath = core.getInput("new_path", { required: true });
    const configPath = core.getInput("config_path") || undefined;

    core.info(`Running EPIC Upgrade Guard compare:`);
    core.info(`Old path: ${oldPath}`);
    core.info(`New path: ${newPath}`);
    if (configPath) {
      core.info(`Custom config path: ${configPath}`);
    }

    // Load epic.toml configuration
    let epicConfig: config.ResolvedEpicConfig;
    try {
      epicConfig = config.loadEpicConfig(configPath);
      core.info(`Loaded epic.toml settings. Fail severity threshold: ${epicConfig.failOnSeverity}`);
    } catch (err: any) {
      core.setFailed(`Failed to validate epic.toml configuration: ${err.message}`);
      return;
    }

    // Check if configuration changed in the pull request
    const configChanged = await checkIfConfigChanged(githubToken);

    // Run layout verification comparison
    const report = await compareAnchorPrograms(oldPath, newPath, epicConfig);

    const findingsCount = report.findings.length;
    core.setOutput("severity", report.severity);
    core.setOutput("findings_count", findingsCount.toString());

    core.info(`Diff completed. Severity: ${report.severity}, Findings: ${findingsCount}`);

    // Generate compact markdown report
    const markdownReport = generateCompactMarkdownReport(report, epicConfig, configChanged);

    // Upsert the comment on GitHub Pull Request
    await upsertPRComment(githubToken, markdownReport);

    // Gate verification
    const severityLevels = ["SAFE", "MINOR", "MAJOR", "CRITICAL"];
    const thresholdIndex = severityLevels.indexOf(epicConfig.failOnSeverity);
    const reportSeverityIndex = severityLevels.indexOf(report.severity);

    if (thresholdIndex !== -1 && reportSeverityIndex !== -1 && reportSeverityIndex >= thresholdIndex) {
      core.setFailed(`EPIC Guard Blocked: Upgrade severity is ${report.severity} (threshold: ${epicConfig.failOnSeverity}).`);
    } else {
      core.info("EPIC Guard approved upgrade.");
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    core.setFailed(`EPIC Upgrade Guard failed: ${message}`);
  }
}

run();


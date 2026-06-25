import fs from "node:fs";

// Colors
const isColorsEnabled = () => {
  if (process.env.NO_COLOR) return false;
  if (!process.stdout.isTTY) return false;
  return true;
};

export const colors = {
  bold: (text: string) => isColorsEnabled() ? `\x1b[1m${text}\x1b[0m` : text,
  dim: (text: string) => isColorsEnabled() ? `\x1b[2m${text}\x1b[0m` : text,
  white: (text: string) => isColorsEnabled() ? `\x1b[1;97m${text}\x1b[0m` : text,
  cyan: (text: string) => isColorsEnabled() ? `\x1b[36m${text}\x1b[0m` : text,
  gray: (text: string) => isColorsEnabled() ? `\x1b[90m${text}\x1b[0m` : text,
  success: (text: string) => isColorsEnabled() ? `\x1b[32m${text}\x1b[0m` : text,
  warning: (text: string) => isColorsEnabled() ? `\x1b[33m${text}\x1b[0m` : text,
  critical: (text: string) => isColorsEnabled() ? `\x1b[31m${text}\x1b[0m` : text,
  info: (text: string) => isColorsEnabled() ? `\x1b[34m${text}\x1b[0m` : text,
  violet: (text: string) => isColorsEnabled() ? `\x1b[35m${text}\x1b[0m` : text,
};

export const DIVIDER = "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━";

let bannerPrinted = false;

export const printBanner = (noBannerFlag: boolean = false) => {
  if (bannerPrinted) return;
  if (noBannerFlag || process.env.EPIC_NO_BANNER === "1") {
    bannerPrinted = true;
    return;
  }
  
  if (!process.stdout.isTTY) {
    bannerPrinted = true;
    return;
  }

  console.log(colors.gray(DIVIDER));
  console.log("");
  console.log(colors.white("EPIC"));
  console.log(colors.dim("Security-first upgrade intelligence for Solana"));
  console.log("");
  console.log(colors.cyan("v0.1.0-beta.2"));
  console.log("");
  console.log(colors.gray(DIVIDER));

  bannerPrinted = true;
};

export const printFinalSignature = () => {
  // Replaced by end summary
};

export const printInitSequence = (steps: string[]) => {
  if (!process.stdout.isTTY) return;
  for (const step of steps) {
    console.log(`${colors.success("✓")} ${step}`);
  }
};

export const printSection = (title: string, data: Record<string, string | number>) => {
  console.log(colors.bold(title));
  console.log("");
  for (const [key, value] of Object.entries(data)) {
    const dots = colors.gray(".".repeat(Math.max(3, 20 - key.length)));
    console.log(`${key} ${dots} ${colors.bold(String(value))}`);
  }
  console.log("");
};

export const formatSeverity = (sev: string) => {
  const s = sev.toUpperCase();
  if (s === "CRITICAL") return colors.critical(s);
  if (s === "HIGH") return colors.warning(s);
  if (s === "MAJOR" || s === "MEDIUM") return colors.warning(s);
  if (s === "WARNING") return colors.warning(s);
  if (s === "INFO") return colors.info(s);
  if (s === "SAFE" || s === "MINOR") return colors.success(s);
  return s;
};

const ruleNames: Record<string, string> = {
  "EPIC-SEC-001": "Owner Validation",
  "EPIC-SEC-002": "Missing Signer Validation",
  "EPIC-SEC-003": "Missing Post-CPI Account Reload",
  "EPIC-SEC-004": "PDA Cryptographic Seed Collision Risk",
  "EPIC-SEC-005": "Arbitrary CPI Target Program Spoofing"
};

export const printRuleFinding = (finding: any) => {
  const sev = formatSeverity(finding.severity);
  const ruleId = colors.violet(finding.rule_id);
  const ruleName = finding.rule_name || ruleNames[finding.rule_id] || finding.rule_id;
  
  console.log(colors.gray("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
  console.log("");
  console.log(sev);
  console.log(ruleId);
  console.log("");
  console.log(colors.bold(ruleName));
  console.log(colors.gray(`${finding.location.file}:${finding.location.line}`));
  console.log("");
  console.log(finding.message);
  console.log("");
  console.log(colors.dim("Recommendation"));
  console.log(finding.recommendation || "Review and validate.");
  console.log("");
};

export const printEndSummary = (rulesExec: number, critical: number, high: number, timeMs: number) => {
  console.log(colors.gray("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
  console.log("");
  console.log(colors.bold("Audit Complete"));
  console.log("");
  
  const printLine = (key: string, val: string | number) => {
    const dots = colors.gray(".".repeat(Math.max(3, 20 - key.length)));
    console.log(`${key} ${dots} ${colors.bold(String(val))}`);
  };
  
  printLine("Rules Executed", rulesExec);
  printLine("Critical", critical);
  printLine("High", high);
  printLine("Time", (timeMs / 1000).toFixed(2) + " s");
  
  console.log("");
  console.log(colors.gray("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
  console.log("");
  console.log(colors.bold("Security Score"));
  
  const deduction = (critical * 10) + (high * 5);
  let score = 100 - deduction;
  if (score < 0) score = 0;
  
  const filled = Math.round(score / 10);
  const unfilled = 10 - filled;
  const bar = colors.cyan("█".repeat(filled)) + colors.gray("░".repeat(unfilled));
  
  console.log(`${bar} ${score}%`);
  console.log("");
  
  if (score < 100) {
    console.log(colors.warning("Repository requires review before deployment."));
  } else {
    console.log(colors.success("Repository is secure and ready for deployment."));
  }
};

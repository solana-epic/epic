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

export const ruleKnowledge: Record<string, { desc: string, fix: string, why: string, historical: string }> = {
  "EPIC-SEC-001": {
    desc: "Missing Owner Validation",
    fix: "Use `#[account(owner = program_id)]` or `Account<'info, T>` which inherently checks ownership.",
    why: "Without owner validation, a malicious user can pass a forged account owned by a different program, bypassing logic checks and potentially draining funds.",
    historical: "Multiple yield aggregators have been drained due to forged state accounts passing checks without owner validation."
  },
  "EPIC-SEC-002": {
    desc: "Missing Signer Validation",
    fix: "Use `Signer<'info>` or `#[account(signer)]`.",
    why: "Without a signer check, an attacker can pass someone else's public key and execute operations on their behalf without their authorization.",
    historical: "A lack of signer validation allows unauthorized withdrawals or parameter tampering."
  },
  "EPIC-SEC-003": {
    desc: "Missing Post-CPI Account Reload",
    fix: "Call:\n\naccount.reload()?\n\nbefore accessing mutated state.",
    why: "After a CPI, Anchor’s in-memory account state can become stale. Reading stale data may produce incorrect logic or security vulnerabilities.",
    historical: "Protocols have shipped stale-account bugs caused by missing reloads after CPIs, leading to double-spends and logic bypasses."
  },
  "EPIC-SEC-004": {
    desc: "PDA Cryptographic Seed Collision Risk",
    fix: "Insert a fixed-length seed or literal delimiter between variable-length seeds.",
    why: "Adjacent variable-length seeds can merge ambiguously, allowing an attacker to craft a PDA collision and spoof accounts.",
    historical: "Improper PDA derivation has allowed attackers to front-run legitimate users by crafting colliding seeds."
  },
  "EPIC-SEC-005": {
    desc: "Arbitrary CPI Target Program Spoofing",
    fix: "Replace:\n\nAccountInfo<'info>\n\nWith:\n\nProgram<'info, Token>\n\nOR\n\nrequire_keys_eq!(\n  token_program.key(),\n  spl_token::ID\n);",
    why: "This guarantees the CPI target cannot be spoofed, preventing malicious code execution from a spoofed program.",
    historical: "Raydium and other major DEXs suffered exploits when attacker-controlled programs were passed into CPIs instead of legitimate ones."
  }
};

export const printRuleFinding = (finding: any) => {
  const sev = formatSeverity(finding.severity);
  const ruleId = colors.violet(finding.rule_id);
  const ruleName = finding.rule_name || ruleNames[finding.rule_id] || finding.rule_id;
  const knowledge = ruleKnowledge[finding.rule_id];
  
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
  if (knowledge) {
    console.log(colors.bold("Suggested Fix"));
    console.log("");
    console.log(knowledge.fix);
    console.log("");
    console.log(colors.bold("Why?"));
    console.log(knowledge.why);
  } else {
    console.log(colors.dim("Recommendation"));
    console.log(finding.recommendation || "Review and validate.");
  }
  console.log("");
};

export const printEndSummary = (rulesExec: number, critical: number, high: number, timeMs: number, nextSteps: string[] = []) => {
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
  
  const deduction = (critical * 20) + (high * 10);
  let score = 100 - deduction;
  if (score < 0) score = 0;
  
  const filled = Math.round(score / 10);
  const unfilled = 10 - filled;
  const bar = colors.cyan("█".repeat(filled)) + colors.gray("░".repeat(unfilled));
  
  console.log(`${bar} ${score} / 100`);
  console.log("");
  console.log(colors.gray("Status"));
  if (score >= 95) {
    console.log(colors.success("Production Ready"));
  } else if (score >= 80) {
    console.log(colors.info("Minor Issues"));
  } else if (score >= 60) {
    console.log(colors.warning("Needs Review"));
  } else if (score >= 40) {
    console.log(colors.warning("High Risk"));
  } else {
    console.log(colors.critical("Unsafe For Deployment"));
  }

  if (nextSteps && nextSteps.length > 0) {
    console.log("");
    console.log(colors.bold("Next Steps"));
    console.log("");
    nextSteps.forEach((step, idx) => {
      console.log(`${idx + 1}. ${step}`);
    });
  }
  console.log("");
};

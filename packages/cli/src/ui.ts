import fs from "node:fs";

// Colors
const isColorsEnabled = () => {
  if (process.env.NO_COLOR) return false;
  if (!process.stdout.isTTY) return false;
  return true;
};

export const colors = {
  gold: (text: string) => isColorsEnabled() ? `\x1b[38;2;214;185;140m${text}\x1b[0m` : text,
  ivory: (text: string) => isColorsEnabled() ? `\x1b[38;2;244;239;230m${text}\x1b[0m` : text,
  gray: (text: string) => isColorsEnabled() ? `\x1b[38;2;156;163;175m${text}\x1b[0m` : text, // Soft gray
  graphite: (text: string) => isColorsEnabled() ? `\x1b[38;2;58;58;58m${text}\x1b[0m` : text,
  success: (text: string) => isColorsEnabled() ? `\x1b[38;2;74;222;128m${text}\x1b[0m` : text,
  warning: (text: string) => isColorsEnabled() ? `\x1b[38;2;251;191;36m${text}\x1b[0m` : text,
  critical: (text: string) => isColorsEnabled() ? `\x1b[38;2;239;68;68m${text}\x1b[0m` : text,
  violet: (text: string) => isColorsEnabled() ? `\x1b[38;2;139;92;246m${text}\x1b[0m` : text,
};

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

  const logo = `
███████╗██████╗ ██╗ ██████╗
██╔════╝██╔══██╗██║██╔════╝
█████╗  ██████╔╝██║██║
██╔══╝  ██╔═══╝ ██║██║
███████╗██║     ██║╚██████╗
╚══════╝╚═╝     ╚═╝ ╚═════╝`;

  console.log(colors.gold(logo.substring(1))); 
  console.log(colors.ivory("EPIC v0.1.0-beta.2"));
  console.log(colors.gray("Know your upgrade before mainnet."));
  console.log(colors.graphite("───────────────────────────────────────────────────────────"));

  bannerPrinted = true;
};

export const printInitSequence = (steps: string[]) => {
  if (!process.stdout.isTTY) return;
  for (const step of steps) {
    console.log(`${colors.success("✓")} ${step}`);
  }
  console.log("");
};

export const printSection = (title: string, data: Record<string, string | number>) => {
  console.log(colors.ivory(title));
  console.log(colors.graphite("────────────────────────"));
  for (const [key, value] of Object.entries(data)) {
    console.log(`${key.padEnd(12)} ${value}`);
  }
  console.log("");
};

export const formatSeverity = (sev: string) => {
  const s = sev.toUpperCase();
  if (s === "CRITICAL" || s === "HIGH") return colors.critical(s);
  if (s === "MAJOR" || s === "MEDIUM") return colors.warning(s);
  if (s === "WARNING") return colors.warning(s);
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
  
  console.log(`${sev.padEnd(isColorsEnabled() ? 19 : 10)} ${ruleId}`);
  console.log(colors.ivory(ruleName));
  console.log(colors.gray("Location"));
  console.log(`${finding.location.file}:${finding.location.line}`);
  console.log(colors.gray("Recommendation"));
  console.log(finding.message);
  console.log("");
};

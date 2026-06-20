import assert from "node:assert/strict";
import { test } from "node:test";
import { generateCompactMarkdownReport } from "../dist/report.js";

// Helper to construct a mock config
const createMockConfig = (failOnSeverity = "MAJOR") => ({
  compareMode: "ast",
  failOnSeverity,
  excludePaths: [],
  enforcePadding: false,
  programs: new Map([
    ["marginfi", {
      name: "marginfi",
      absolutePath: "/workspace/programs/marginfi",
      programId: "MFv2...",
      overrides: [
        {
          account: "Bank",
          finding: "PADDING_REPURPOSE",
          field: "reserved",
          action: "allow",
          note: "Replaced padding safely.",
          used: true
        }
      ]
    }]
  ])
});

test("report: generates approved status banner when clean", () => {
  const report = {
    oldProgramPath: "/old",
    newProgramPath: "/new",
    severity: "SAFE",
    findings: []
  };
  const config = createMockConfig();

  const md = generateCompactMarkdownReport(report, config, false);
  assert.match(md, /🟢 EPIC Guard: APPROVED/);
  assert.match(md, /Upgrade checks approved/);
  assert.match(md, /No structural changes/);
  assert.match(md, /No action required/);
});

test("report: generates overrides active banner when warnings are muted", () => {
  const report = {
    oldProgramPath: "/old",
    newProgramPath: "/new",
    severity: "SAFE",
    findings: [
      {
        severity: "SAFE", // Overridden from CRITICAL
        account: "Bank",
        kind: "PADDING_REPURPOSE",
        field: { name: "reserved" },
        oldSize: 100,
        newSize: 100
      }
    ]
  };
  const config = createMockConfig();

  const md = generateCompactMarkdownReport(report, config, false);
  assert.match(md, /🟡 EPIC Guard: APPROVED WITH OVERRIDES/);
  assert.match(md, /Applied Layout Overrides/);
  assert.match(md, /Replaced padding safely/);
  assert.match(md, /`CRITICAL` ──► `SAFE`/);
});

test("report: generates blocked status banner when threshold is exceeded", () => {
  const report = {
    oldProgramPath: "/old",
    newProgramPath: "/new",
    severity: "MAJOR",
    findings: [
      {
        severity: "MAJOR", // Not overridden
        account: "Bank",
        kind: "FIELD_ADDED",
        field: { name: "maker_rebate" },
        oldSize: 100,
        newSize: 108
      }
    ]
  };
  const config = createMockConfig("MAJOR"); // Will block since report severity is MAJOR

  const md = generateCompactMarkdownReport(report, config, false);
  assert.match(md, /🔴 EPIC Guard: UPGRADE BLOCKED/);
  assert.match(md, /Upgrade checks failed because layout changes exceed/);
});

test("report: injects warning banner when epic.toml changed", () => {
  const report = {
    oldProgramPath: "/old",
    newProgramPath: "/new",
    severity: "SAFE",
    findings: []
  };
  const config = createMockConfig();

  const md = generateCompactMarkdownReport(report, config, true); // configChanged = true
  assert.match(md, /🟢 EPIC Guard: APPROVED/);
  assert.match(md, /UPGRADE CONFIGURATION GATE MODIFIED/);
  assert.match(md, /This Pull Request contains changes to `epic.toml`/);
});

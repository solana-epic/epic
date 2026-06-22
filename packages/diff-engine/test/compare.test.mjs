import assert from "node:assert/strict";
import path from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import { compareAnchorPrograms, createUpgradeIntelligence, formatHumanReport } from "../dist/index.js";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
const fixture = (name) => path.join(repoRoot, "fixtures", name);

test("detects field additions as warning upgrade risks", async () => {
  const report = await compareAnchorPrograms(fixture("old-position"), fixture("new-added-field"));

  assert.equal(report.severity, "WARNING");
  assert.equal(report.findings.length, 1);
  assert.deepEqual(report.findings[0], {
    severity: "WARNING",
    account: "Position",
    kind: "FIELD_ADDED",
    field: {
      name: "amount",
      newType: "u64"
    },
    oldSize: 40,
    newSize: 48
  });
});

test("detects field removals as critical upgrade risks", async () => {
  const report = await compareAnchorPrograms(fixture("new-added-field"), fixture("new-removed-field"));

  assert.equal(report.severity, "CRITICAL");
  assert.equal(report.findings.length, 2);

  const removedFinding = report.findings.find(f => f.kind === "FIELD_REMOVED");
  assert.ok(removedFinding);
  assert.deepEqual(removedFinding.field, {
    name: "amount",
    oldType: "u64"
  });

  const sizeFinding = report.findings.find(f => f.kind === "SIZE_REDUCED");
  assert.ok(sizeFinding);
  assert.equal(sizeFinding.oldSize, 48);
  assert.equal(sizeFinding.newSize, 40);
});

test("detects field reordering as critical upgrade risks", async () => {
  const report = await compareAnchorPrograms(fixture("new-added-field"), fixture("new-reordered-field"));

  assert.equal(report.severity, "CRITICAL");
  assert.equal(report.findings.length, 1);
  assert.equal(report.findings[0].kind, "FIELD_REORDERED");
});

test("detects type changes as critical layout drift", async () => {
  const report = await compareAnchorPrograms(fixture("new-added-field"), fixture("new-type-change"));

  assert.equal(report.severity, "CRITICAL");
  assert.equal(report.findings.length, 1);
  assert.deepEqual(report.findings[0], {
    severity: "CRITICAL",
    account: "Position",
    kind: "TYPE_CHANGED",
    field: {
      name: "amount",
      oldType: "u64",
      newType: "u128"
    },
    oldSize: 48,
    newSize: 56
  });
});

test("formats human-readable reports", async () => {
  const report = await compareAnchorPrograms(fixture("new-added-field"), fixture("new-reordered-field"));
  const output = formatHumanReport(report);

  assert.match(output, /EPIC UPGRADE REPORT/);
  assert.match(output, /Program: Position/);
  assert.match(output, /Severity: CRITICAL/);
  assert.match(output, /Field Reordered/);
  assert.match(output, /Risk Category:\nField Reorder/);
  assert.match(output, /Affected Surface:\n- Existing Accounts\n- Client SDKs\n- Indexers\n- IDLs/);
  assert.match(output, /Do not reorder persisted fields; append new fields at the end or migrate into a new account layout\./);
});

test("creates upgrade intelligence for type width serialization breaks", async () => {
  const report = await compareAnchorPrograms(fixture("new-added-field"), fixture("new-type-change"));
  const intelligence = createUpgradeIntelligence(report);

  assert.deepEqual(intelligence, {
    severity: "CRITICAL",
    items: [
      {
        account: "Position",
        field: "amount",
        change: "u64 -> u128",
        findingKind: "TYPE_CHANGED",
        severity: "CRITICAL",
        riskCategory: "Serialization Break",
        affectedSurface: ["Existing Accounts", "Client SDKs", "Indexers", "IDLs"],
        recommendation: "Create migration instruction before upgrade."
      }
    ]
  });
});

test("creates upgrade intelligence for account expansion and shrink", async () => {
  const expansion = createUpgradeIntelligence(await compareAnchorPrograms(fixture("old-position"), fixture("new-added-field")));
  const shrink = createUpgradeIntelligence(await compareAnchorPrograms(fixture("new-added-field"), fixture("new-removed-field")));

  assert.equal(expansion.items[0].riskCategory, "Account Expansion");
  assert.deepEqual(expansion.items[0].affectedSurface, ["Existing Accounts", "IDLs", "Client SDKs"]);
  assert.equal(shrink.items[0].riskCategory, "Account Shrink");
  assert.deepEqual(shrink.items[0].affectedSurface, ["Existing Accounts", "Client SDKs", "Indexers", "IDLs"]);
});

test("creates upgrade intelligence for dynamic type introductions", async () => {
  const report = await compareAnchorPrograms(fixture("old-position"), fixture("new-dynamic-field"));
  const intelligence = createUpgradeIntelligence(report);

  assert.equal(intelligence.items[0].riskCategory, "Dynamic Type Introduction");
  assert.equal(intelligence.items[0].field, "collateral");
  assert.equal(
    intelligence.items[0].recommendation,
    "Avoid introducing dynamic persisted fields without bounded sizing and an explicit migration plan."
  );
});

test("applies configuration overrides to downgrade severity", async () => {
  const mockConfig = {
    compareMode: "ast",
    failOnSeverity: "CRITICAL",
    excludePaths: [],
    enforcePadding: false,
    programs: new Map([
      ["new-added-field", {
        name: "new-added-field",
        absolutePath: fixture("new-added-field"),
        programId: "dRif...",
        overrides: [
          {
            account: "Position",
            finding: "FIELD_ADDED",
            field: "amount",
            action: "downgrade",
            severity: "SAFE",
            note: "Muted because it has preallocated headroom.",
            used: false
          }
        ]
      }]
    ])
  };

  const report = await compareAnchorPrograms(fixture("old-position"), fixture("new-added-field"), mockConfig);
  assert.equal(report.findings[0].severity, "SAFE");
  assert.equal(report.severity, "SAFE");
  assert.equal(mockConfig.programs.get("new-added-field").overrides[0].used, true);
});

test("ignores overrides for banned findings (FIELD_REMOVED)", async () => {
  const mockConfig = {
    compareMode: "ast",
    failOnSeverity: "CRITICAL",
    excludePaths: [],
    enforcePadding: false,
    programs: new Map([
      ["new-removed-field", {
        name: "new-removed-field",
        absolutePath: fixture("new-removed-field"),
        programId: "dRif...",
        overrides: [
          {
            account: "Position",
            finding: "FIELD_REMOVED",
            field: "amount",
            action: "allow",
            note: "Attempting to override field removal",
            used: false
          }
        ]
      }]
    ])
  };

  const report = await compareAnchorPrograms(fixture("new-added-field"), fixture("new-removed-field"), mockConfig);
  // Re-ordering and removals remain CRITICAL even if someone bypassed TOML checks
  assert.equal(report.findings[0].severity, "CRITICAL");
  assert.equal(report.severity, "CRITICAL");
});


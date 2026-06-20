import assert from "node:assert/strict";
import { test } from "node:test";
import { compareAccountLayouts } from "../dist/compare.js";
import { analyzeAnchorProject } from "@epic/parser";
import fs from "node:fs";
import path from "node:path";
import os from "node:os";

test("Issue 2.1: Reordering is detected even when fields are simultaneously added/removed", () => {
  const oldAccount = {
    accountId: "user::User",
    name: "User",
    namespace: "user",
    byteSize: 24,
    byteSizeIncludesDiscriminator: true,
    abiFingerprint: "hash1",
    hasDynamicSize: false,
    layoutWarnings: [],
    fields: [
      { name: "x", type: "u64", byteSize: 8, dynamic: false },
      { name: "y", type: "u64", byteSize: 8, dynamic: false }
    ],
    filePath: "lib.rs"
  };

  const newAccount = {
    accountId: "user::User",
    name: "User",
    namespace: "user",
    byteSize: 32,
    byteSizeIncludesDiscriminator: true,
    abiFingerprint: "hash2",
    hasDynamicSize: false,
    layoutWarnings: [],
    fields: [
      { name: "y", type: "u64", byteSize: 8, dynamic: false },
      { name: "x", type: "u64", byteSize: 8, dynamic: false },
      { name: "z", type: "u64", byteSize: 8, dynamic: false } // Swapped x/y and added z
    ],
    filePath: "lib.rs"
  };

  const oldProgram = { projectPath: ".", accounts: [oldAccount] };
  const newProgram = { projectPath: ".", accounts: [newAccount] };

  const report = compareAccountLayouts(oldProgram, newProgram);
  
  // Verify that it successfully detects and reports FIELD_REORDERED as CRITICAL
  const kinds = report.findings.map(f => f.kind);
  const reorderFinding = report.findings.find(f => f.kind === "FIELD_REORDERED");

  assert.ok(kinds.includes("FIELD_ADDED"));
  assert.ok(kinds.includes("FIELD_REORDERED"), "Reordering should be reported");
  assert.equal(reorderFinding.severity, "CRITICAL");
  assert.equal(report.severity, "CRITICAL", "Overall severity should be CRITICAL due to reordering");
});

test("Issue 2.2: Middle-inserted field additions are mapped to CRITICAL severity", () => {
  const oldAccount = {
    accountId: "data::Data",
    name: "Data",
    namespace: "data",
    byteSize: 17,
    byteSizeIncludesDiscriminator: true,
    abiFingerprint: "hash1",
    hasDynamicSize: false,
    layoutWarnings: [],
    fields: [
      { name: "active", type: "bool", byteSize: 1, dynamic: false },
      { name: "count", type: "u64", byteSize: 8, dynamic: false }
    ],
    filePath: "lib.rs"
  };

  const newAccount = {
    accountId: "data::Data",
    name: "Data",
    namespace: "data",
    byteSize: 18,
    byteSizeIncludesDiscriminator: true,
    abiFingerprint: "hash2",
    hasDynamicSize: false,
    layoutWarnings: [],
    fields: [
      { name: "active", type: "bool", byteSize: 1, dynamic: false },
      { name: "val", type: "u8", byteSize: 1, dynamic: false }, // Inserted in the middle!
      { name: "count", type: "u64", byteSize: 8, dynamic: false }
    ],
    filePath: "lib.rs"
  };

  const oldProgram = { projectPath: ".", accounts: [oldAccount] };
  const newProgram = { projectPath: ".", accounts: [newAccount] };

  const report = compareAccountLayouts(oldProgram, newProgram);

  // The engine must map middle-insert to CRITICAL
  const finding = report.findings.find(f => f.kind === "FIELD_ADDED");
  assert.ok(finding);
  assert.equal(finding.severity, "CRITICAL", "Middle insertion should be CRITICAL");
  assert.equal(report.severity, "CRITICAL", "Overall report severity should be CRITICAL due to middle insertion");
});

test("Issue 2.3: Enums with variants of varying sizes are correctly mapped as hasDynamicSize: true", async () => {
  const mockIdl = {
    name: "test_enum",
    version: "0.1.0",
    instructions: [],
    accounts: [
      {
        name: "StateAccount",
        type: {
          kind: "struct",
          fields: [
            { name: "state", type: { defined: "ProgramState" } },
            { name: "total", type: "u64" }
          ]
        }
      }
    ],
    types: [
      {
        name: "ProgramState",
        type: {
          kind: "enum",
          variants: [
            { name: "Uninitialized" }, // Variant 0: 0B
            { name: "Active", fields: [{ name: "owner", type: "publicKey" }] } // Variant 1: 32B
          ]
        }
      }
    ]
  };

  const tempIdlPath = path.join(os.tmpdir(), `idl-${Date.now()}.json`);
  fs.writeFileSync(tempIdlPath, JSON.stringify(mockIdl, null, 2));

  try {
    const result = await analyzeAnchorProject(tempIdlPath);
    const account = result.accounts[0];
    
    assert.equal(account.name, "StateAccount");
    // Verify that the engine resolves enum with varying variants as dynamic
    assert.equal(account.hasDynamicSize, true, "Should be flagged as dynamic: true");
    assert.equal(account.byteSize, 8 + (1 + 32) + 8); // 8 disc + 33 max enum + 8 u64 = 49 bytes
    
    // Check warning code matches DYNAMIC_TYPE
    assert.equal(account.layoutWarnings[0].code, "DYNAMIC_TYPE");
    assert.match(account.layoutWarnings[0].message, /Dynamic size detected/);
  } finally {
    fs.rmSync(tempIdlPath, { force: true });
  }
});

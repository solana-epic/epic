import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { test } from "node:test";
import { compareAnchorProjects, compareAccountSets } from "../dist/diff.js";
import { analyzeAnchorProject } from "../dist/project.js";
import { AnalysisError, parseAccountStructs } from "../dist/rust.js";

test("fails closed on unknown nested types", () => {
  const source = `
    #[account]
    pub struct Position {
      pub config: NestedConfig,
    }
  `;

  assert.throws(
    () => parseAccountStructs(source, "/tmp/position.rs"),
    (error) => {
      assert.equal(error instanceof AnalysisError, true);
      assert.equal(error.account, "Position");
      assert.equal(error.field, "config");
      assert.equal(error.type, "NestedConfig");
      assert.match(error.message, /Unable to resolve type "NestedConfig"/);
      assert.match(error.message, /EPIC cannot safely calculate layout size/);
      return true;
    }
  );
});

test("detects field reorder as critical ABI change", () => {
  const oldAccounts = parseAccountStructs(
    `
      #[account]
      pub struct Position {
        pub a: u64,
        pub b: u8,
        pub c: Pubkey,
      }
    `,
    "/tmp/old.rs"
  );
  const newAccounts = parseAccountStructs(
    `
      #[account]
      pub struct Position {
        pub b: u8,
        pub a: u64,
        pub c: Pubkey,
      }
    `,
    "/tmp/old.rs"
  );

  const diffs = compareAccountSets(oldAccounts, newAccounts);

  assert.equal(diffs.length, 1);
  assert.equal(diffs[0].fieldReordered, true);
  assert.equal(diffs[0].abiFingerprintChanged, true);
  assert.equal(diffs[0].riskLevel, "Critical");
  assert.equal(diffs[0].complexity, "Critical");
  assert.deepEqual(diffs[0].reasons, ["Field Reorder Detected", "ABI fingerprint changed"]);
});

test("keeps duplicate account names separate across program namespaces", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "epic-trust-"));
  await mkdir(path.join(root, "program-a"), { recursive: true });
  await mkdir(path.join(root, "program-b"), { recursive: true });

  await writeFile(
    path.join(root, "program-a", "lib.rs"),
    `
      #[account]
      pub struct Config {
        pub owner: Pubkey,
      }
    `
  );
  await writeFile(
    path.join(root, "program-b", "lib.rs"),
    `
      #[account]
      pub struct Config {
        pub bump: u8,
      }
    `
  );

  const result = await analyzeAnchorProject(root);

  assert.deepEqual(
    result.accounts.map((account) => account.accountId).sort(),
    ["program-a/lib.rs::Config", "program-b/lib.rs::Config"]
  );
});

test("does not collide duplicate account names when diffing multi-program workspaces", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "epic-trust-diff-"));
  const oldRoot = path.join(root, "old");
  const newRoot = path.join(root, "new");
  await mkdir(path.join(oldRoot, "program-a"), { recursive: true });
  await mkdir(path.join(oldRoot, "program-b"), { recursive: true });
  await mkdir(path.join(newRoot, "program-a"), { recursive: true });
  await mkdir(path.join(newRoot, "program-b"), { recursive: true });

  await writeFile(path.join(oldRoot, "program-a", "lib.rs"), `#[account] pub struct Config { pub owner: Pubkey, }`);
  await writeFile(path.join(oldRoot, "program-b", "lib.rs"), `#[account] pub struct Config { pub bump: u8, }`);
  await writeFile(path.join(newRoot, "program-a", "lib.rs"), `#[account] pub struct Config { pub owner: Pubkey, pub score: u64, }`);
  await writeFile(path.join(newRoot, "program-b", "lib.rs"), `#[account] pub struct Config { pub bump: u8, }`);

  const report = await compareAnchorProjects(oldRoot, newRoot);

  assert.equal(report.accountDiffs.length, 1);
  assert.equal(report.accountDiffs[0].accountId, "program-a/lib.rs::Config");
  assert.equal(report.accountDiffs[0].name, "Config");
  assert.deepEqual(report.accountDiffs[0].addedFields.map((field) => field.name), ["score"]);
});

test("marks dynamic types with warnings instead of claiming exact static layout", () => {
  const accounts = parseAccountStructs(
    `
      #[account]
      pub struct Position {
        pub label: String,
        pub scores: Vec<u64>,
        pub owners: HashSet<Pubkey>,
      }
    `,
    "/tmp/dynamic.rs"
  );

  assert.equal(accounts[0].hasDynamicSize, true);
  assert.equal(accounts[0].layoutWarnings.length, 3);
  assert.deepEqual(
    accounts[0].layoutWarnings.map((warning) => warning.type),
    ["String", "Vec<u64>", "HashSet<Pubkey>"]
  );
});

test("changes ABI fingerprint for insertion, removal, and type change", () => {
  const oldAccount = parseAccountStructs(
    `#[account] pub struct Position { pub owner: Pubkey, pub score: u64, }`,
    "/tmp/fingerprint.rs"
  )[0];
  const inserted = parseAccountStructs(
    `#[account] pub struct Position { pub owner: Pubkey, pub score: u64, pub bump: u8, }`,
    "/tmp/fingerprint.rs"
  )[0];
  const removed = parseAccountStructs(
    `#[account] pub struct Position { pub owner: Pubkey, }`,
    "/tmp/fingerprint.rs"
  )[0];
  const typed = parseAccountStructs(
    `#[account] pub struct Position { pub owner: Pubkey, pub score: i64, }`,
    "/tmp/fingerprint.rs"
  )[0];

  assert.notEqual(oldAccount.abiFingerprint, inserted.abiFingerprint);
  assert.notEqual(oldAccount.abiFingerprint, removed.abiFingerprint);
  assert.notEqual(oldAccount.abiFingerprint, typed.abiFingerprint);
});

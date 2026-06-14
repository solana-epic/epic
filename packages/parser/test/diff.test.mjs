import assert from "node:assert/strict";
import { test } from "node:test";
import { compareAccountSets } from "../dist/diff.js";

const filePath = "/tmp/lib.rs";

test("detects added fields, size changes, migration requirement, and medium risk", () => {
  const diffs = compareAccountSets(
    [
      {
        name: "Position",
        byteSize: 40,
        byteSizeIncludesDiscriminator: true,
        fields: [{ name: "owner", type: "Pubkey", byteSize: 32 }],
        filePath
      }
    ],
    [
      {
        name: "Position",
        byteSize: 48,
        byteSizeIncludesDiscriminator: true,
        fields: [
          { name: "owner", type: "Pubkey", byteSize: 32 },
          { name: "score", type: "u64", byteSize: 8 }
        ],
        filePath
      }
    ]
  );

  assert.equal(diffs.length, 1);
  assert.equal(diffs[0].name, "Position");
  assert.equal(diffs[0].oldSize, 40);
  assert.equal(diffs[0].newSize, 48);
  assert.deepEqual(diffs[0].addedFields.map((field) => field.name), ["score"]);
  assert.equal(diffs[0].migrationRequired, true);
  assert.equal(diffs[0].riskLevel, "Medium");
  assert.deepEqual(diffs[0].recommendations, [
    "Reallocate existing accounts",
    "Top up rent",
    "Regenerate IDL",
    "Rebuild clients"
  ]);
});

test("detects removed fields and type changes as high risk", () => {
  const diffs = compareAccountSets(
    [
      {
        name: "Position",
        byteSize: 49,
        byteSizeIncludesDiscriminator: true,
        fields: [
          { name: "owner", type: "Pubkey", byteSize: 32 },
          { name: "score", type: "u64", byteSize: 8 },
          { name: "bump", type: "u8", byteSize: 1 }
        ],
        filePath
      }
    ],
    [
      {
        name: "Position",
        byteSize: 48,
        byteSizeIncludesDiscriminator: true,
        fields: [
          { name: "owner", type: "Pubkey", byteSize: 32 },
          { name: "score", type: "i64", byteSize: 8 }
        ],
        filePath
      }
    ]
  );

  assert.equal(diffs.length, 1);
  assert.deepEqual(diffs[0].removedFields.map((field) => field.name), ["bump"]);
  assert.deepEqual(diffs[0].typeChangedFields, [
    {
      name: "score",
      oldType: "u64",
      newType: "i64",
      oldByteSize: 8,
      newByteSize: 8
    }
  ]);
  assert.equal(diffs[0].riskLevel, "High");
});

test("detects added and removed account definitions", () => {
  const diffs = compareAccountSets(
    [
      {
        name: "Legacy",
        byteSize: 16,
        byteSizeIncludesDiscriminator: true,
        fields: [{ name: "count", type: "u64", byteSize: 8 }],
        filePath
      }
    ],
    [
      {
        name: "Vault",
        byteSize: 40,
        byteSizeIncludesDiscriminator: true,
        fields: [{ name: "authority", type: "Pubkey", byteSize: 32 }],
        filePath
      }
    ]
  );

  assert.deepEqual(
    diffs.map((diff) => ({ name: diff.name, status: diff.status, riskLevel: diff.riskLevel })),
    [
      { name: "Legacy", status: "removed", riskLevel: "High" },
      { name: "Vault", status: "added", riskLevel: "Low" }
    ]
  );
});

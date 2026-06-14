import assert from "node:assert/strict";
import { test } from "node:test";
import { compareAccountSets, toMachineReadableReport } from "../dist/diff.js";

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
  assert.equal(diffs[0].sizeDelta, 8);
  assert.equal(diffs[0].additionalBytesRequired, 8);
  assert.deepEqual(diffs[0].addedFields.map((field) => field.name), ["score"]);
  assert.equal(diffs[0].migrationRequired, true);
  assert.equal(diffs[0].riskLevel, "Medium");
  assert.equal(diffs[0].complexity, "Medium");
  assert.equal(diffs[0].realloc.required, true);
  assert.equal(diffs[0].realloc.suggestedAction, "account.realloc(48, false)");
  assert.deepEqual(diffs[0].rentImpact, {
    status: "Increased",
    estimatedAdditionalBytes: 8,
    exactLamports: null,
    futureHook: "RPC rent exemption lookup"
  });
  assert.deepEqual(diffs[0].upgradePlan, [
    "Reallocate Position account",
    "Top up rent exemption",
    "Regenerate IDL",
    "Rebuild TypeScript clients",
    "Run migration tests"
  ]);
  assert.deepEqual(diffs[0].recommendations, [
    "Reallocate existing accounts",
    "Top up rent exemption",
    "Regenerate IDL",
    "Rebuild clients"
  ]);
});

test("detects removed fields and type changes as critical complexity", () => {
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
  assert.equal(diffs[0].riskLevel, "Critical");
  assert.equal(diffs[0].complexity, "Critical");
  assert.equal(diffs[0].realloc.required, false);
  assert.equal(diffs[0].rentImpact.status, "Decreased");
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

test("detects size shrinkage as high complexity when layout size decreases", () => {
  const diffs = compareAccountSets(
    [
      {
        name: "Position",
        byteSize: 48,
        byteSizeIncludesDiscriminator: true,
        fields: [{ name: "owner", type: "Pubkey", byteSize: 32 }],
        filePath
      }
    ],
    [
      {
        name: "Position",
        byteSize: 40,
        byteSizeIncludesDiscriminator: true,
        fields: [{ name: "owner", type: "Pubkey", byteSize: 32 }],
        filePath
      }
    ]
  );

  assert.equal(diffs.length, 1);
  assert.equal(diffs[0].sizeDelta, -8);
  assert.equal(diffs[0].additionalBytesRequired, 0);
  assert.equal(diffs[0].riskLevel, "High");
  assert.equal(diffs[0].complexity, "High");
  assert.equal(diffs[0].rentImpact.status, "Decreased");
});

test("projects upgrade reports into machine-readable JSON shape", () => {
  const report = {
    oldProjectPath: "/old",
    newProjectPath: "/new",
    accountsChanged: 1,
    overallRisk: "Medium",
    accountDiffs: compareAccountSets(
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
    )
  };

  const json = toMachineReadableReport(report);

  assert.equal(json.accountsChanged, 1);
  assert.equal(json.overallRisk, "medium");
  assert.deepEqual(json.accounts[0], {
    account: "Position",
    accountId: "/tmp/lib.rs::Position",
    namespace: "/tmp/lib.rs",
    status: "changed",
    oldSize: 40,
    newSize: 48,
    delta: 8,
    additionalBytesRequired: 8,
    migrationRequired: true,
    risk: "medium",
    complexity: "medium",
    reallocRequired: true,
    rentImpact: "increased",
    estimatedAdditionalBytes: 8,
    addedFields: [{ name: "score", type: "u64" }],
    removedFields: [],
    typeChanges: [],
    abiFingerprintChanged: true,
    oldAbiFingerprint: JSON.stringify({
      account: "Position",
      fields: [{ name: "owner", type: "Pubkey" }]
    }),
    newAbiFingerprint: JSON.stringify({
      account: "Position",
      fields: [
        { name: "owner", type: "Pubkey" },
        { name: "score", type: "u64" }
      ]
    }),
    fieldReordered: false,
    reasons: ["Field insertion detected", "Account size changed", "ABI fingerprint changed"],
    dynamicSize: false,
    warnings: [],
    upgradePlan: [
      "Reallocate Position account",
      "Top up rent exemption",
      "Regenerate IDL",
      "Rebuild TypeScript clients",
      "Run migration tests"
    ]
  });
});

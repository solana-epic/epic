import assert from "node:assert/strict";
import { test } from "node:test";
import { fileURLToPath } from "node:url";
import {
  simulateUpgrade,
  StaticUpgradeSimulationRunner,
  toMachineReadableSimulation
} from "../dist/simulation.js";

const fixtureRoot = fileURLToPath(new URL("../../../fixtures/", import.meta.url));
const oldFixture = `${fixtureRoot}upgrade-old`;
const newFixture = `${fixtureRoot}upgrade-new`;

test("simulates an additive upgrade from existing project fixtures", async () => {
  const simulation = await simulateUpgrade(oldFixture, newFixture);

  assert.equal(simulation.mode, "static");
  assert.equal(simulation.adapter, "static-analysis");
  assert.equal(simulation.affectedAccounts.length, 1);
  assert.deepEqual(simulation.affectedAccounts[0], {
    name: "Position",
    status: "changed",
    oldSize: 40,
    newSize: 48,
    sizeDelta: 8,
    migrationRequired: true,
    riskLevel: "Medium",
    complexity: "Medium"
  });
  assert.deepEqual(simulation.reallocRequirements, [
    {
      account: "Position",
      required: true,
      oldSize: 40,
      newSize: 48,
      additionalBytesRequired: 8,
      suggestedAction: "account.realloc(48, false)"
    }
  ]);
  assert.equal(simulation.riskLevel, "Medium");
  assert.equal(simulation.riskScore, 50);
  assert.deepEqual(simulation.estimatedRentIncrease, {
    additionalBytes: 8,
    exactLamports: null,
    calculationMode: "byte-delta-only",
    futureHook: "RPC rent exemption lookup"
  });
  assert.equal(simulation.bankrunReady, false);
});

test("supports injectable simulation runners for future Bankrun integration", async () => {
  const runner = {
    async simulate() {
      return {
        mode: "static",
        adapter: "bankrun",
        oldProjectPath: "/old",
        newProjectPath: "/new",
        affectedAccounts: [],
        reallocRequirements: [],
        migrationPlan: [],
        riskLevel: "None",
        riskScore: 0,
        estimatedRentIncrease: {
          additionalBytes: 0,
          exactLamports: null,
          calculationMode: "byte-delta-only",
          futureHook: "RPC rent exemption lookup"
        },
        bankrunReady: true,
        bankrunHook: "custom runner",
        report: {
          oldProjectPath: "/old",
          newProjectPath: "/new",
          accountsChanged: 0,
          accountDiffs: [],
          overallRisk: "None"
        }
      };
    }
  };

  const simulation = await simulateUpgrade("/old", "/new", runner);

  assert.equal(simulation.adapter, "bankrun");
  assert.equal(simulation.bankrunReady, true);
});

test("projects simulation into machine-readable output", async () => {
  const simulation = await new StaticUpgradeSimulationRunner().simulate(
    oldFixture,
    newFixture
  );

  assert.deepEqual(toMachineReadableSimulation(simulation), {
    mode: "static",
    adapter: "static-analysis",
    affectedAccounts: [
      {
        name: "Position",
        status: "changed",
        oldSize: 40,
        newSize: 48,
        sizeDelta: 8,
        migrationRequired: true,
        riskLevel: "Medium",
        complexity: "Medium"
      }
    ],
    reallocRequirements: [
      {
        account: "Position",
        required: true,
        oldSize: 40,
        newSize: 48,
        additionalBytesRequired: 8,
        suggestedAction: "account.realloc(48, false)"
      }
    ],
    migrationPlan: [
      "Reallocate Position account",
      "Top up rent exemption",
      "Regenerate IDL",
      "Rebuild TypeScript clients",
      "Run migration tests"
    ],
    risk: "medium",
    riskScore: 50,
    estimatedRentIncrease: {
      additionalBytes: 8,
      exactLamports: null,
      calculationMode: "byte-delta-only",
      futureHook: "RPC rent exemption lookup"
    },
    bankrunReady: false,
    bankrunHook: "UpgradeSimulationRunner can be backed by Bankrun execution in a future adapter."
  });
});

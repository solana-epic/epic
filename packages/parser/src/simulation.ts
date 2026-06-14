import type {
  AffectedAccount,
  EstimatedRentIncrease,
  MachineReadableUpgradeSimulation,
  ReallocRequirement,
  RiskLevel,
  UpgradeReadinessReport,
  UpgradeSimulation
} from "./index.js";
import { compareAnchorProjects } from "./diff.js";

const RISK_SCORES: Record<RiskLevel, number> = {
  None: 0,
  Low: 25,
  Medium: 50,
  High: 75,
  Critical: 100
};

export interface UpgradeSimulationRunner {
  simulate(oldProjectPath: string, newProjectPath: string): Promise<UpgradeSimulation>;
}

export class StaticUpgradeSimulationRunner implements UpgradeSimulationRunner {
  async simulate(oldProjectPath: string, newProjectPath: string): Promise<UpgradeSimulation> {
    const report = await compareAnchorProjects(oldProjectPath, newProjectPath);
    return buildStaticSimulation(report);
  }
}

export async function simulateUpgrade(
  oldProjectPath: string,
  newProjectPath: string,
  runner: UpgradeSimulationRunner = new StaticUpgradeSimulationRunner()
): Promise<UpgradeSimulation> {
  return runner.simulate(oldProjectPath, newProjectPath);
}

export function toMachineReadableSimulation(
  simulation: UpgradeSimulation
): MachineReadableUpgradeSimulation {
  return {
    mode: simulation.mode,
    adapter: simulation.adapter,
    affectedAccounts: simulation.affectedAccounts,
    reallocRequirements: simulation.reallocRequirements,
    migrationPlan: simulation.migrationPlan,
    risk: simulation.riskLevel.toLowerCase() as Lowercase<RiskLevel>,
    riskScore: simulation.riskScore,
    estimatedRentIncrease: simulation.estimatedRentIncrease,
    bankrunReady: simulation.bankrunReady,
    bankrunHook: simulation.bankrunHook
  };
}

function buildStaticSimulation(report: UpgradeReadinessReport): UpgradeSimulation {
  const affectedAccounts: AffectedAccount[] = report.accountDiffs.map((diff) => ({
    name: diff.name,
    status: diff.status,
    oldSize: diff.oldSize,
    newSize: diff.newSize,
    sizeDelta: diff.sizeDelta,
    migrationRequired: diff.migrationRequired,
    riskLevel: diff.riskLevel,
    complexity: diff.complexity
  }));

  const reallocRequirements: ReallocRequirement[] = report.accountDiffs.map((diff) => ({
    account: diff.name,
    required: diff.realloc.required,
    oldSize: diff.oldSize,
    newSize: diff.newSize,
    additionalBytesRequired: diff.additionalBytesRequired,
    suggestedAction: diff.realloc.suggestedAction
  }));

  return {
    mode: "static",
    adapter: "static-analysis",
    oldProjectPath: report.oldProjectPath,
    newProjectPath: report.newProjectPath,
    affectedAccounts,
    reallocRequirements,
    migrationPlan: buildMigrationPlan(report),
    riskLevel: report.overallRisk,
    riskScore: RISK_SCORES[report.overallRisk],
    estimatedRentIncrease: estimateRentIncrease(report),
    bankrunReady: false,
    bankrunHook: "UpgradeSimulationRunner can be backed by Bankrun execution in a future adapter.",
    report
  };
}

function buildMigrationPlan(report: UpgradeReadinessReport): string[] {
  const steps: string[] = [];

  for (const diff of report.accountDiffs) {
    steps.push(...diff.upgradePlan);
  }

  return Array.from(new Set(steps));
}

function estimateRentIncrease(report: UpgradeReadinessReport): EstimatedRentIncrease {
  const additionalBytes = report.accountDiffs.reduce((sum, diff) => {
    return sum + diff.rentImpact.estimatedAdditionalBytes;
  }, 0);

  return {
    additionalBytes,
    exactLamports: null,
    calculationMode: "byte-delta-only",
    futureHook: "RPC rent exemption lookup"
  };
}

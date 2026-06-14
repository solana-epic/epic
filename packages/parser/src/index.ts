export type AccountField = {
  name: string;
  type: string;
  byteSize: number;
  dynamic: boolean;
  note?: string;
};

export type LayoutWarning = {
  code: "DYNAMIC_TYPE";
  message: string;
  account: string;
  field: string;
  type: string;
  filePath: string;
};

export type AccountStruct = {
  accountId: string;
  name: string;
  namespace: string;
  byteSize: number;
  byteSizeIncludesDiscriminator: true;
  abiFingerprint: string;
  hasDynamicSize: boolean;
  layoutWarnings: LayoutWarning[];
  fields: AccountField[];
  filePath: string;
};

export type AnalyzeResult = {
  projectPath: string;
  accounts: AccountStruct[];
};

export type RiskLevel = "None" | "Low" | "Medium" | "High" | "Critical";

export type MigrationComplexity = "Low" | "Medium" | "High" | "Critical";

export type RentImpactStatus = "Unchanged" | "Increased" | "Decreased" | "Unknown";

export type RentImpact = {
  status: RentImpactStatus;
  estimatedAdditionalBytes: number;
  exactLamports: number | null;
  futureHook: "RPC rent exemption lookup";
};

export type ReallocGuidance = {
  required: boolean;
  newSize: number | null;
  payerRequired: boolean;
  zeroInit: boolean;
  suggestedAction: string | null;
};

export type FieldTypeChange = {
  name: string;
  oldType: string;
  newType: string;
  oldByteSize: number;
  newByteSize: number;
};

export type AccountDiffStatus = "added" | "removed" | "changed";

export type AccountDiff = {
  accountId: string;
  name: string;
  namespace: string;
  status: AccountDiffStatus;
  oldSize: number | null;
  newSize: number | null;
  sizeDelta: number | null;
  additionalBytesRequired: number;
  addedFields: AccountField[];
  removedFields: AccountField[];
  typeChangedFields: FieldTypeChange[];
  sizeChanged: boolean;
  abiFingerprintChanged: boolean;
  oldAbiFingerprint: string | null;
  newAbiFingerprint: string | null;
  fieldReordered: boolean;
  reasons: string[];
  layoutWarnings: LayoutWarning[];
  hasDynamicSize: boolean;
  migrationRequired: boolean;
  riskLevel: RiskLevel;
  complexity: MigrationComplexity;
  realloc: ReallocGuidance;
  rentImpact: RentImpact;
  recommendations: string[];
  upgradePlan: string[];
};

export type UpgradeReadinessReport = {
  oldProjectPath: string;
  newProjectPath: string;
  accountsChanged: number;
  accountDiffs: AccountDiff[];
  overallRisk: RiskLevel;
};

export type MachineReadableAccountDiff = {
  account: string;
  accountId: string;
  namespace: string;
  status: AccountDiffStatus;
  oldSize: number | null;
  newSize: number | null;
  delta: number | null;
  additionalBytesRequired: number;
  migrationRequired: boolean;
  risk: Lowercase<RiskLevel>;
  complexity: Lowercase<MigrationComplexity>;
  reallocRequired: boolean;
  rentImpact: Lowercase<RentImpactStatus>;
  estimatedAdditionalBytes: number;
  addedFields: Array<{ name: string; type: string }>;
  removedFields: Array<{ name: string; type: string }>;
  typeChanges: Array<{ name: string; oldType: string; newType: string }>;
  abiFingerprintChanged: boolean;
  oldAbiFingerprint: string | null;
  newAbiFingerprint: string | null;
  fieldReordered: boolean;
  reasons: string[];
  dynamicSize: boolean;
  warnings: LayoutWarning[];
  upgradePlan: string[];
};

export type MachineReadableUpgradeReport = {
  accountsChanged: number;
  overallRisk: Lowercase<RiskLevel>;
  accounts: MachineReadableAccountDiff[];
};

export type SimulationMode = "static";

export type SimulationAdapter = "static-analysis" | "bankrun";

export type AffectedAccount = {
  name: string;
  status: AccountDiffStatus;
  oldSize: number | null;
  newSize: number | null;
  sizeDelta: number | null;
  migrationRequired: boolean;
  riskLevel: RiskLevel;
  complexity: MigrationComplexity;
};

export type ReallocRequirement = {
  account: string;
  required: boolean;
  oldSize: number | null;
  newSize: number | null;
  additionalBytesRequired: number;
  suggestedAction: string | null;
};

export type EstimatedRentIncrease = {
  additionalBytes: number;
  exactLamports: number | null;
  calculationMode: "byte-delta-only";
  futureHook: "RPC rent exemption lookup";
};

export type UpgradeSimulation = {
  mode: SimulationMode;
  adapter: SimulationAdapter;
  oldProjectPath: string;
  newProjectPath: string;
  affectedAccounts: AffectedAccount[];
  reallocRequirements: ReallocRequirement[];
  migrationPlan: string[];
  riskLevel: RiskLevel;
  riskScore: number;
  estimatedRentIncrease: EstimatedRentIncrease;
  bankrunReady: boolean;
  bankrunHook: string;
  report: UpgradeReadinessReport;
};

export type MachineReadableUpgradeSimulation = {
  mode: SimulationMode;
  adapter: SimulationAdapter;
  affectedAccounts: AffectedAccount[];
  reallocRequirements: ReallocRequirement[];
  migrationPlan: string[];
  risk: Lowercase<RiskLevel>;
  riskScore: number;
  estimatedRentIncrease: EstimatedRentIncrease;
  bankrunReady: boolean;
  bankrunHook: string;
};

export { analyzeAnchorProject } from "./project.js";
export { compareAccountSets, compareAnchorProjects, toMachineReadableReport } from "./diff.js";
export { AnalysisError } from "./rust.js";
export { simulateUpgrade, toMachineReadableSimulation } from "./simulation.js";

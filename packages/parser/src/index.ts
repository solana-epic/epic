export type AccountField = {
  name: string;
  type: string;
  byteSize: number | null;
  note?: string;
};

export type AccountStruct = {
  name: string;
  byteSize: number;
  byteSizeIncludesDiscriminator: true;
  fields: AccountField[];
  filePath: string;
};

export type AnalyzeResult = {
  projectPath: string;
  accounts: AccountStruct[];
};

export type RiskLevel = "None" | "Low" | "Medium" | "High";

export type FieldTypeChange = {
  name: string;
  oldType: string;
  newType: string;
  oldByteSize: number | null;
  newByteSize: number | null;
};

export type AccountDiffStatus = "added" | "removed" | "changed";

export type AccountDiff = {
  name: string;
  status: AccountDiffStatus;
  oldSize: number | null;
  newSize: number | null;
  addedFields: AccountField[];
  removedFields: AccountField[];
  typeChangedFields: FieldTypeChange[];
  sizeChanged: boolean;
  migrationRequired: boolean;
  riskLevel: RiskLevel;
  recommendations: string[];
};

export type UpgradeReadinessReport = {
  oldProjectPath: string;
  newProjectPath: string;
  accountsChanged: number;
  accountDiffs: AccountDiff[];
  overallRisk: RiskLevel;
};

export { analyzeAnchorProject } from "./project.js";
export { compareAccountSets, compareAnchorProjects } from "./diff.js";

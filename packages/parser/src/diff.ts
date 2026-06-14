import type {
  AccountDiff,
  AccountField,
  AccountStruct,
  AnalyzeResult,
  FieldTypeChange,
  MachineReadableUpgradeReport,
  MigrationComplexity,
  ReallocGuidance,
  RiskLevel,
  UpgradeReadinessReport
} from "./index.js";
import { analyzeAnchorProject } from "./project.js";
import { defaultRentEstimator } from "./rent.js";

const RISK_ORDER: RiskLevel[] = ["None", "Low", "Medium", "High", "Critical"];

export async function compareAnchorProjects(
  oldProjectPath: string,
  newProjectPath: string
): Promise<UpgradeReadinessReport> {
  const [oldProject, newProject] = await Promise.all([
    analyzeAnchorProject(oldProjectPath),
    analyzeAnchorProject(newProjectPath)
  ]);

  return buildUpgradeReadinessReport(oldProject, newProject);
}

export function compareAccountSets(
  oldAccounts: AccountStruct[],
  newAccounts: AccountStruct[]
): AccountDiff[] {
  const oldByName = mapAccountsByName(oldAccounts);
  const newByName = mapAccountsByName(newAccounts);
  const accountNames = Array.from(new Set([...oldByName.keys(), ...newByName.keys()])).sort();
  const diffs: AccountDiff[] = [];

  for (const name of accountNames) {
    const oldAccount = oldByName.get(name);
    const newAccount = newByName.get(name);

    if (!oldAccount && newAccount) {
      diffs.push(buildAddedAccountDiff(newAccount));
      continue;
    }

    if (oldAccount && !newAccount) {
      diffs.push(buildRemovedAccountDiff(oldAccount));
      continue;
    }

    if (oldAccount && newAccount) {
      const changed = buildChangedAccountDiff(oldAccount, newAccount);

      if (changed) {
        diffs.push(changed);
      }
    }
  }

  return diffs;
}

export function toMachineReadableReport(report: UpgradeReadinessReport): MachineReadableUpgradeReport {
  return {
    accountsChanged: report.accountsChanged,
    overallRisk: lowerRisk(report.overallRisk),
    accounts: report.accountDiffs.map((diff) => ({
      account: diff.name,
      accountId: diff.accountId,
      namespace: diff.namespace,
      status: diff.status,
      oldSize: diff.oldSize,
      newSize: diff.newSize,
      delta: diff.sizeDelta,
      additionalBytesRequired: diff.additionalBytesRequired,
      migrationRequired: diff.migrationRequired,
      risk: lowerRisk(diff.riskLevel),
      complexity: lowerComplexity(diff.complexity),
      reallocRequired: diff.realloc.required,
      rentImpact: lowerRentImpact(diff.rentImpact.status),
      estimatedAdditionalBytes: diff.rentImpact.estimatedAdditionalBytes,
      addedFields: diff.addedFields.map((field) => ({ name: field.name, type: field.type })),
      removedFields: diff.removedFields.map((field) => ({ name: field.name, type: field.type })),
      typeChanges: diff.typeChangedFields.map((field) => ({
        name: field.name,
        oldType: field.oldType,
        newType: field.newType
      })),
      abiFingerprintChanged: diff.abiFingerprintChanged,
      oldAbiFingerprint: diff.oldAbiFingerprint,
      newAbiFingerprint: diff.newAbiFingerprint,
      fieldReordered: diff.fieldReordered,
      reasons: diff.reasons,
      dynamicSize: diff.hasDynamicSize,
      warnings: diff.layoutWarnings,
      upgradePlan: diff.upgradePlan
    }))
  };
}

function buildUpgradeReadinessReport(
  oldProject: AnalyzeResult,
  newProject: AnalyzeResult
): UpgradeReadinessReport {
  const accountDiffs = compareAccountSets(oldProject.accounts, newProject.accounts);

  return {
    oldProjectPath: oldProject.projectPath,
    newProjectPath: newProject.projectPath,
    accountsChanged: accountDiffs.length,
    accountDiffs,
    overallRisk: highestRisk(accountDiffs.map((diff) => diff.riskLevel))
  };
}

function buildAddedAccountDiff(account: AccountStruct): AccountDiff {
  return {
    accountId: accountKey(account),
    name: account.name,
    namespace: accountNamespace(account),
    status: "added",
    oldSize: null,
    newSize: account.byteSize,
    sizeDelta: null,
    additionalBytesRequired: 0,
    addedFields: account.fields,
    removedFields: [],
    typeChangedFields: [],
    sizeChanged: true,
    abiFingerprintChanged: true,
    oldAbiFingerprint: null,
    newAbiFingerprint: accountFingerprint(account),
    fieldReordered: false,
    reasons: ["Account introduced"],
    layoutWarnings: accountLayoutWarnings(account),
    hasDynamicSize: accountHasDynamicSize(account),
    migrationRequired: false,
    riskLevel: "Low",
    complexity: "Low",
    realloc: reallocGuidance(null),
    rentImpact: defaultRentEstimator.estimate({ oldSize: null, newSize: account.byteSize }),
    recommendations: uniqueRecommendations([
      "Update account initialization flows",
      "Regenerate IDL",
      "Rebuild clients"
    ]),
    upgradePlan: [
      `Add initialization path for ${account.name} accounts`,
      "Regenerate IDL",
      "Rebuild TypeScript clients",
      "Run migration tests"
    ]
  };
}

function buildRemovedAccountDiff(account: AccountStruct): AccountDiff {
  return {
    accountId: accountKey(account),
    name: account.name,
    namespace: accountNamespace(account),
    status: "removed",
    oldSize: account.byteSize,
    newSize: null,
    sizeDelta: null,
    additionalBytesRequired: 0,
    addedFields: [],
    removedFields: account.fields,
    typeChangedFields: [],
    sizeChanged: true,
    abiFingerprintChanged: true,
    oldAbiFingerprint: accountFingerprint(account),
    newAbiFingerprint: null,
    fieldReordered: false,
    reasons: ["Account removed"],
    layoutWarnings: accountLayoutWarnings(account),
    hasDynamicSize: accountHasDynamicSize(account),
    migrationRequired: true,
    riskLevel: "High",
    complexity: "High",
    realloc: reallocGuidance(null),
    rentImpact: defaultRentEstimator.estimate({ oldSize: account.byteSize, newSize: null }),
    recommendations: uniqueRecommendations([
      "Plan explicit state deprecation or migration",
      "Remove dependent instructions and clients",
      "Regenerate IDL",
      "Rebuild clients"
    ]),
    upgradePlan: [
      `Plan deprecation or migration for ${account.name} accounts`,
      "Remove dependent instructions and clients",
      "Regenerate IDL",
      "Rebuild TypeScript clients",
      "Run migration tests"
    ]
  };
}

function buildChangedAccountDiff(
  oldAccount: AccountStruct,
  newAccount: AccountStruct
): AccountDiff | null {
  const oldFields = mapFieldsByName(oldAccount.fields);
  const newFields = mapFieldsByName(newAccount.fields);
  const addedFields = newAccount.fields.filter((field) => !oldFields.has(field.name));
  const removedFields = oldAccount.fields.filter((field) => !newFields.has(field.name));
  const typeChangedFields: FieldTypeChange[] = [];

  for (const oldField of oldAccount.fields) {
    const newField = newFields.get(oldField.name);

    if (newField && oldField.type !== newField.type) {
      typeChangedFields.push({
        name: oldField.name,
        oldType: oldField.type,
        newType: newField.type,
        oldByteSize: oldField.byteSize,
        newByteSize: newField.byteSize
      });
    }
  }

  const sizeChanged = oldAccount.byteSize !== newAccount.byteSize;
  const sizeDelta = newAccount.byteSize - oldAccount.byteSize;
  const abiFingerprintChanged = accountFingerprint(oldAccount) !== accountFingerprint(newAccount);
  const fieldReordered = isFieldReorder(oldAccount.fields, newAccount.fields);

  if (
    addedFields.length === 0 &&
    removedFields.length === 0 &&
    typeChangedFields.length === 0 &&
    !sizeChanged &&
    !abiFingerprintChanged
  ) {
    return null;
  }

  const migrationRequired = sizeChanged || addedFields.length > 0 || removedFields.length > 0 || typeChangedFields.length > 0 || fieldReordered;
  const riskLevel = riskForChangedAccount({
    oldSize: oldAccount.byteSize,
    newSize: newAccount.byteSize,
    addedFields,
    removedFields,
    typeChangedFields,
    fieldReordered
  });
  const complexity = complexityForChangedAccount({
    oldSize: oldAccount.byteSize,
    newSize: newAccount.byteSize,
    addedFields,
    removedFields,
    typeChangedFields,
    fieldReordered
  });

  return {
    accountId: accountKey(oldAccount),
    name: oldAccount.name,
    namespace: accountNamespace(oldAccount),
    status: "changed",
    oldSize: oldAccount.byteSize,
    newSize: newAccount.byteSize,
    sizeDelta,
    additionalBytesRequired: Math.max(sizeDelta, 0),
    addedFields,
    removedFields,
    typeChangedFields,
    sizeChanged,
    abiFingerprintChanged,
    oldAbiFingerprint: accountFingerprint(oldAccount),
    newAbiFingerprint: accountFingerprint(newAccount),
    fieldReordered,
    reasons: reasonsForChangedAccount({
      addedFields,
      removedFields,
      typeChangedFields,
      sizeChanged,
      fieldReordered,
      abiFingerprintChanged
    }),
    layoutWarnings: [...accountLayoutWarnings(oldAccount), ...accountLayoutWarnings(newAccount)],
    hasDynamicSize: accountHasDynamicSize(oldAccount) || accountHasDynamicSize(newAccount),
    migrationRequired,
    riskLevel,
    complexity,
    realloc: reallocGuidance(newAccount.byteSize, sizeDelta),
    rentImpact: defaultRentEstimator.estimate({
      oldSize: oldAccount.byteSize,
      newSize: newAccount.byteSize
    }),
    recommendations: recommendationsForChangedAccount({
      oldSize: oldAccount.byteSize,
      newSize: newAccount.byteSize,
      addedFields,
      removedFields,
      typeChangedFields,
      fieldReordered
    }),
    upgradePlan: upgradePlanForChangedAccount(oldAccount.name, {
      oldSize: oldAccount.byteSize,
      newSize: newAccount.byteSize,
      addedFields,
      removedFields,
      typeChangedFields,
      fieldReordered
    })
  };
}

function riskForChangedAccount(input: {
  oldSize: number;
  newSize: number;
  addedFields: AccountField[];
  removedFields: AccountField[];
  typeChangedFields: FieldTypeChange[];
  fieldReordered?: boolean;
}): RiskLevel {
  if (input.fieldReordered) {
    return "Critical";
  }

  if (destructiveChangeCount(input) > 1) {
    return "Critical";
  }

  if (input.removedFields.length > 0 || input.typeChangedFields.length > 0 || input.newSize < input.oldSize) {
    return "High";
  }

  if (input.addedFields.length > 0 || input.newSize > input.oldSize) {
    return "Medium";
  }

  return "Low";
}

function complexityForChangedAccount(input: {
  oldSize: number;
  newSize: number;
  addedFields: AccountField[];
  removedFields: AccountField[];
  typeChangedFields: FieldTypeChange[];
  fieldReordered?: boolean;
}): MigrationComplexity {
  if (input.fieldReordered) {
    return "Critical";
  }

  if (destructiveChangeCount(input) > 1) {
    return "Critical";
  }

  if (input.removedFields.length > 0 || input.typeChangedFields.length > 0 || input.newSize < input.oldSize) {
    return "High";
  }

  if (input.addedFields.length > 0 || input.newSize > input.oldSize) {
    return "Medium";
  }

  return "Low";
}

function recommendationsForChangedAccount(input: {
  oldSize: number;
  newSize: number;
  addedFields: AccountField[];
  removedFields: AccountField[];
  typeChangedFields: FieldTypeChange[];
  fieldReordered?: boolean;
}): string[] {
  const recommendations: string[] = [];

  if (input.fieldReordered) {
    recommendations.push("Treat field reorder as an incompatible layout change", "Plan explicit state migration");
  }

  if (input.addedFields.length > 0 || input.newSize > input.oldSize) {
    recommendations.push("Reallocate existing accounts", "Top up rent exemption");
  }

  if (input.removedFields.length > 0 || input.newSize < input.oldSize) {
    recommendations.push("Plan explicit state migration", "Verify existing account deserialization");
  }

  if (input.typeChangedFields.length > 0) {
    recommendations.push("Backfill or transform existing account data", "Verify instruction compatibility");
  }

  recommendations.push("Regenerate IDL", "Rebuild clients");
  return uniqueRecommendations(recommendations);
}

function upgradePlanForChangedAccount(accountName: string, input: {
  oldSize: number;
  newSize: number;
  addedFields: AccountField[];
  removedFields: AccountField[];
  typeChangedFields: FieldTypeChange[];
  fieldReordered?: boolean;
}): string[] {
  const steps: string[] = [];

  if (input.fieldReordered) {
    steps.push(`Plan incompatible layout migration for ${accountName}`, `Verify ${accountName} account decoding against existing data`);
  }

  if (input.newSize > input.oldSize) {
    steps.push(`Reallocate ${accountName} account`, "Top up rent exemption");
  }

  if (input.removedFields.length > 0 || input.newSize < input.oldSize) {
    steps.push(`Plan state migration for ${accountName}`);
  }

  if (input.typeChangedFields.length > 0) {
    steps.push(`Transform existing ${accountName} account data`);
  }

  steps.push("Regenerate IDL", "Rebuild TypeScript clients", "Run migration tests");
  return uniqueRecommendations(steps);
}

function reallocGuidance(newSize: number | null, sizeDelta = 0): ReallocGuidance {
  if (newSize === null || sizeDelta <= 0) {
    return {
      required: false,
      newSize,
      payerRequired: false,
      zeroInit: false,
      suggestedAction: null
    };
  }

  return {
    required: true,
    newSize,
    payerRequired: true,
    zeroInit: false,
    suggestedAction: `account.realloc(${newSize}, false)`
  };
}

function destructiveChangeCount(input: {
  oldSize: number;
  newSize: number;
  removedFields: AccountField[];
  typeChangedFields: FieldTypeChange[];
}): number {
  return input.removedFields.length + input.typeChangedFields.length + (input.newSize < input.oldSize ? 1 : 0);
}

function mapAccountsByName(accounts: AccountStruct[]): Map<string, AccountStruct> {
  return new Map(accounts.map((account) => [accountKey(account), account]));
}

function mapFieldsByName(fields: AccountField[]): Map<string, AccountField> {
  return new Map(fields.map((field) => [field.name, field]));
}

function highestRisk(risks: RiskLevel[]): RiskLevel {
  return risks.reduce<RiskLevel>((highest, risk) => {
    return RISK_ORDER.indexOf(risk) > RISK_ORDER.indexOf(highest) ? risk : highest;
  }, "None");
}

function uniqueRecommendations(recommendations: string[]): string[] {
  return Array.from(new Set(recommendations));
}

function accountKey(account: AccountStruct): string {
  return account.accountId ?? `${account.namespace ?? account.filePath}::${account.name}`;
}

function accountNamespace(account: AccountStruct): string {
  return account.namespace ?? account.filePath;
}

function accountFingerprint(account: AccountStruct): string {
  return account.abiFingerprint ?? JSON.stringify({
    account: account.name,
    fields: account.fields.map((field) => ({ name: field.name, type: field.type }))
  });
}

function accountLayoutWarnings(account: AccountStruct) {
  return account.layoutWarnings ?? [];
}

function accountHasDynamicSize(account: AccountStruct): boolean {
  return account.hasDynamicSize ?? account.fields.some((field) => field.dynamic);
}

function isFieldReorder(oldFields: AccountField[], newFields: AccountField[]): boolean {
  if (oldFields.length !== newFields.length) {
    return false;
  }

  const oldSignatures = oldFields.map((field) => `${field.name}:${field.type}`).sort();
  const newSignatures = newFields.map((field) => `${field.name}:${field.type}`).sort();

  if (!oldSignatures.every((signature, index) => signature === newSignatures[index])) {
    return false;
  }

  return oldFields.some((field, index) => {
    const newField = newFields[index];
    return field.name !== newField.name || field.type !== newField.type;
  });
}

function reasonsForChangedAccount(input: {
  addedFields: AccountField[];
  removedFields: AccountField[];
  typeChangedFields: FieldTypeChange[];
  sizeChanged: boolean;
  fieldReordered: boolean;
  abiFingerprintChanged: boolean;
}): string[] {
  const reasons: string[] = [];

  if (input.fieldReordered) {
    reasons.push("Field Reorder Detected");
  }

  if (input.addedFields.length > 0) {
    reasons.push("Field insertion detected");
  }

  if (input.removedFields.length > 0) {
    reasons.push("Field removal detected");
  }

  if (input.typeChangedFields.length > 0) {
    reasons.push("Field type change detected");
  }

  if (input.sizeChanged) {
    reasons.push("Account size changed");
  }

  if (input.abiFingerprintChanged) {
    reasons.push("ABI fingerprint changed");
  }

  return uniqueRecommendations(reasons);
}

function lowerRisk(risk: RiskLevel): Lowercase<RiskLevel> {
  return risk.toLowerCase() as Lowercase<RiskLevel>;
}

function lowerComplexity(complexity: MigrationComplexity): Lowercase<MigrationComplexity> {
  return complexity.toLowerCase() as Lowercase<MigrationComplexity>;
}

function lowerRentImpact(status: "Unchanged" | "Increased" | "Decreased" | "Unknown"): Lowercase<typeof status> {
  return status.toLowerCase() as Lowercase<typeof status>;
}

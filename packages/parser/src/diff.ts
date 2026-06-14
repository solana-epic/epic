import type {
  AccountDiff,
  AccountField,
  AccountStruct,
  AnalyzeResult,
  FieldTypeChange,
  RiskLevel,
  UpgradeReadinessReport
} from "./index.js";
import { analyzeAnchorProject } from "./project.js";

const RISK_ORDER: RiskLevel[] = ["None", "Low", "Medium", "High"];

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
    name: account.name,
    status: "added",
    oldSize: null,
    newSize: account.byteSize,
    addedFields: account.fields,
    removedFields: [],
    typeChangedFields: [],
    sizeChanged: true,
    migrationRequired: false,
    riskLevel: "Low",
    recommendations: uniqueRecommendations([
      "Update account initialization flows",
      "Regenerate IDL",
      "Rebuild clients"
    ])
  };
}

function buildRemovedAccountDiff(account: AccountStruct): AccountDiff {
  return {
    name: account.name,
    status: "removed",
    oldSize: account.byteSize,
    newSize: null,
    addedFields: [],
    removedFields: account.fields,
    typeChangedFields: [],
    sizeChanged: true,
    migrationRequired: true,
    riskLevel: "High",
    recommendations: uniqueRecommendations([
      "Plan explicit state deprecation or migration",
      "Remove dependent instructions and clients",
      "Regenerate IDL",
      "Rebuild clients"
    ])
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

  if (
    addedFields.length === 0 &&
    removedFields.length === 0 &&
    typeChangedFields.length === 0 &&
    !sizeChanged
  ) {
    return null;
  }

  const migrationRequired = sizeChanged || addedFields.length > 0 || removedFields.length > 0 || typeChangedFields.length > 0;
  const riskLevel = riskForChangedAccount({
    oldSize: oldAccount.byteSize,
    newSize: newAccount.byteSize,
    addedFields,
    removedFields,
    typeChangedFields
  });

  return {
    name: oldAccount.name,
    status: "changed",
    oldSize: oldAccount.byteSize,
    newSize: newAccount.byteSize,
    addedFields,
    removedFields,
    typeChangedFields,
    sizeChanged,
    migrationRequired,
    riskLevel,
    recommendations: recommendationsForChangedAccount({
      oldSize: oldAccount.byteSize,
      newSize: newAccount.byteSize,
      addedFields,
      removedFields,
      typeChangedFields
    })
  };
}

function riskForChangedAccount(input: {
  oldSize: number;
  newSize: number;
  addedFields: AccountField[];
  removedFields: AccountField[];
  typeChangedFields: FieldTypeChange[];
}): RiskLevel {
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
}): string[] {
  const recommendations: string[] = [];

  if (input.addedFields.length > 0 || input.newSize > input.oldSize) {
    recommendations.push("Reallocate existing accounts", "Top up rent");
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

function mapAccountsByName(accounts: AccountStruct[]): Map<string, AccountStruct> {
  return new Map(accounts.map((account) => [account.name, account]));
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

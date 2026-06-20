import type { AccountField, AccountStruct, AnalyzeResult, config } from "@epic/parser";
import { analyzeAnchorProject } from "@epic/parser";
import { classifyFindings, highestSeverity } from "./classify.js";
import { resolveFindingsWithConfig, findProgramName } from "./resolve.js";
import path from "node:path";

export type Severity = "SAFE" | "MINOR" | "MAJOR" | "CRITICAL";

export type DiffFindingKind = "FIELD_ADDED" | "FIELD_REMOVED" | "FIELD_REORDERED" | "TYPE_CHANGED";

export type FieldChange = {
  name: string;
  oldType?: string;
  newType?: string;
};

export type DiffFinding = {
  severity: Severity;
  account: string;
  kind: DiffFindingKind;
  field?: FieldChange;
  oldSize: number;
  newSize: number;
};

export type DiffReport = {
  oldProgramPath: string;
  newProgramPath: string;
  severity: Severity;
  findings: DiffFinding[];
};

export async function compareAnchorPrograms(
  oldProgramDir: string,
  newProgramDir: string,
  cfg?: config.ResolvedEpicConfig
): Promise<DiffReport> {
  const [oldProgram, newProgram] = await Promise.all([
    analyzeAnchorProject(oldProgramDir, cfg?.excludePaths),
    analyzeAnchorProject(newProgramDir, cfg?.excludePaths)
  ]);

  return compareAccountLayouts(oldProgram, newProgram, cfg);
}

export function compareAccountLayouts(
  oldProgram: AnalyzeResult,
  newProgram: AnalyzeResult,
  cfg?: config.ResolvedEpicConfig
): DiffReport {
  const oldAccounts = mapAccountsByName(oldProgram.accounts);
  const newAccounts = mapAccountsByName(newProgram.accounts);
  const accountNames = Array.from(new Set([...oldAccounts.keys(), ...newAccounts.keys()])).sort();
  const findings: DiffFinding[] = [];

  for (const accountName of accountNames) {
    const oldAccount = oldAccounts.get(accountName);
    const newAccount = newAccounts.get(accountName);

    if (!oldAccount || !newAccount) {
      continue;
    }

    findings.push(...compareAccount(oldAccount, newAccount));
  }

  let classifiedFindings = classifyFindings(findings);

  if (cfg) {
    const programName = findProgramName(newProgram.projectPath, cfg) || path.basename(newProgram.projectPath);
    classifiedFindings = resolveFindingsWithConfig(programName, classifiedFindings, cfg);
  }

  return {
    oldProgramPath: oldProgram.projectPath,
    newProgramPath: newProgram.projectPath,
    severity: highestSeverity(classifiedFindings.map((finding) => finding.severity)),
    findings: classifiedFindings
  };
}

function compareAccount(oldAccount: AccountStruct, newAccount: AccountStruct): DiffFinding[] {
  const findings: DiffFinding[] = [];
  const oldFields = mapFieldsByName(oldAccount.fields);
  const newFields = mapFieldsByName(newAccount.fields);

  for (const oldField of oldAccount.fields) {
    const newField = newFields.get(oldField.name);

    if (!newField) {
      findings.push({
        severity: "CRITICAL",
        account: oldAccount.name,
        kind: "FIELD_REMOVED",
        field: {
          name: oldField.name,
          oldType: oldField.type
        },
        oldSize: oldAccount.byteSize,
        newSize: newAccount.byteSize
      });
      continue;
    }

    if (oldField.type !== newField.type) {
      findings.push({
        severity: "CRITICAL",
        account: oldAccount.name,
        kind: "TYPE_CHANGED",
        field: {
          name: oldField.name,
          oldType: oldField.type,
          newType: newField.type
        },
        oldSize: oldAccount.byteSize,
        newSize: newAccount.byteSize
      });
    }
  }

  for (const newField of newAccount.fields) {
    if (!oldFields.has(newField.name)) {
      const newFieldIndex = newAccount.fields.findIndex(f => f.name === newField.name);
      const trailingFields = newAccount.fields.slice(newFieldIndex + 1);
      const isMiddleInsertion = trailingFields.some(f => oldFields.has(f.name));

      findings.push({
        severity: isMiddleInsertion ? "CRITICAL" : "MAJOR",
        account: newAccount.name,
        kind: "FIELD_ADDED",
        field: {
          name: newField.name,
          newType: newField.type
        },
        oldSize: oldAccount.byteSize,
        newSize: newAccount.byteSize
      });
    }
  }

  const intersectingFieldsInOld = oldAccount.fields.filter((f) => newFields.has(f.name)).map((f) => f.name);
  const intersectingFieldsInNew = newAccount.fields.filter((f) => oldFields.has(f.name)).map((f) => f.name);
  const hasReordering = intersectingFieldsInOld.some((name, index) => intersectingFieldsInNew[index] !== name);

  if (hasReordering) {
    findings.push({
      severity: "CRITICAL",
      account: oldAccount.name,
      kind: "FIELD_REORDERED",
      oldSize: oldAccount.byteSize,
      newSize: newAccount.byteSize
    });
  }

  return findings;
}

function mapAccountsByName(accounts: AccountStruct[]): Map<string, AccountStruct> {
  return new Map(accounts.map((account) => [account.name, account]));
}

function mapFieldsByName(fields: AccountField[]): Map<string, AccountField> {
  return new Map(fields.map((field) => [field.name, field]));
}

function hasSameFieldNames(oldFields: AccountField[], newFields: AccountField[]): boolean {
  if (oldFields.length !== newFields.length) {
    return false;
  }

  const newFieldNames = new Set(newFields.map((field) => field.name));
  return oldFields.every((field) => newFieldNames.has(field.name));
}

function hasFieldReordering(oldFields: AccountField[], newFields: AccountField[]): boolean {
  return oldFields.some((field, index) => newFields[index]?.name !== field.name);
}

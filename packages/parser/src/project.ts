import { readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";
import type { AccountStruct, AnalyzeResult } from "./index.js";
import { parseAccountStructs } from "./rust.js";

const RUST_EXTENSION = ".rs";

export async function analyzeAnchorProject(projectPath: string): Promise<AnalyzeResult> {
  const resolvedProjectPath = path.resolve(projectPath);
  const rustFiles = await findRustFiles(resolvedProjectPath);
  const accounts: AccountStruct[] = [];

  for (const filePath of rustFiles) {
    const source = await readFile(filePath, "utf8");
    const parsedAccounts = parseAccountStructs(source, filePath).map((account) => {
      const namespace = path.relative(resolvedProjectPath, filePath) || path.basename(filePath);

      return {
        ...account,
        namespace,
        accountId: `${namespace}::${account.name}`
      };
    });

    accounts.push(...parsedAccounts);
  }

  return {
    projectPath: resolvedProjectPath,
    accounts: accounts.sort((left, right) => {
      if (left.namespace === right.namespace) {
        return left.name.localeCompare(right.name);
      }

      return left.namespace.localeCompare(right.namespace);
    })
  };
}

async function findRustFiles(rootPath: string): Promise<string[]> {
  const rootStats = await stat(rootPath);

  if (rootStats.isFile()) {
    return path.extname(rootPath) === RUST_EXTENSION ? [rootPath] : [];
  }

  const files: string[] = [];
  await walk(rootPath, files);
  return files;
}

async function walk(currentPath: string, files: string[]): Promise<void> {
  const entries = await readdir(currentPath, { withFileTypes: true });

  for (const entry of entries) {
    if (entry.name === "target" || entry.name === "node_modules" || entry.name === ".git") {
      continue;
    }

    const entryPath = path.join(currentPath, entry.name);

    if (entry.isDirectory()) {
      await walk(entryPath, files);
      continue;
    }

    if (entry.isFile() && path.extname(entry.name) === RUST_EXTENSION) {
      files.push(entryPath);
    }
  }
}

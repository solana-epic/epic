import { readdir, readFile, stat } from "node:fs/promises";
import path from "node:path";
import { createHash } from "node:crypto";
import picomatch from "picomatch";
import type { AccountStruct, AnalyzeResult, AccountField, ProgramIr, ProgramIrInstruction, ProgramIrAccount, ProgramIrType } from "./index.js";
import { parseAccountStructs, parseAllRawStructs, parseInstructions } from "./rust.js";

const RUST_EXTENSION = ".rs";

export async function analyzeAnchorProject(
  projectPath: string,
  excludePaths: string[] = []
): Promise<AnalyzeResult> {
  const resolvedProjectPath = path.resolve(projectPath);

  if (resolvedProjectPath.endsWith(".json")) {
    return analyzeAnchorIdl(resolvedProjectPath);
  }

  let rustFiles = await findRustFiles(resolvedProjectPath);
  
  if (excludePaths && excludePaths.length > 0) {
    const isMatch = picomatch(excludePaths, { dot: true });
    rustFiles = rustFiles.filter(filePath => {
      const relativePath = path.relative(resolvedProjectPath, filePath);
      // Also try absolute matching to handle monorepo configurations cleanly
      return !isMatch(relativePath) && !isMatch(filePath);
    });
  }

  // 1. Build a registry of all raw structs across all Rust files
  const typesRegistry = new Map<string, any>();
  for (const filePath of rustFiles) {
    try {
      const source = await readFile(filePath, "utf8");
      const rawStructs = parseAllRawStructs(source, filePath);
      for (const struct of rawStructs) {
        typesRegistry.set(struct.name, struct);
      }
    } catch {
      // Ignore reading issues during registration scan
    }
  }

  const accounts: AccountStruct[] = [];
  const instructions: any[] = [];

  for (const filePath of rustFiles) {
    const source = await readFile(filePath, "utf8");
    const parsedAccounts = parseAccountStructs(source, filePath, typesRegistry).map((account) => {
      const namespace = path.relative(resolvedProjectPath, filePath) || path.basename(filePath);

      return {
        ...account,
        namespace,
        accountId: `${namespace}::${account.name}`
      };
    });

    accounts.push(...parsedAccounts);

    const parsedInstructions = parseInstructions(source, filePath);
    instructions.push(...parsedInstructions);
  }

  return {
    projectPath: resolvedProjectPath,
    accounts: accounts.sort((left, right) => {
      if (left.namespace === right.namespace) {
        return left.name.localeCompare(right.name);
      }

      return left.namespace.localeCompare(right.namespace);
    }),
    instructions
  };
}

export function extractAccountsFromIdl(idl: any, idlPath: string): AccountStruct[] {
  const accounts: AccountStruct[] = [];
  const typesRegistry = new Map<string, any>();

  if (idl.types) {
    for (const typeDef of idl.types) {
      typesRegistry.set(typeDef.name, typeDef);
    }
  }

  const idlAccounts = idl.accounts || [];
  const namespace = idl.metadata?.name || idl.name || path.basename(idlPath, ".json");

  for (const idlAccount of idlAccounts) {
    let idlFields: any[] = [];
    if (idlAccount.type && idlAccount.type.fields) {
      idlFields = idlAccount.type.fields;
    } else {
      const typeDef = typesRegistry.get(idlAccount.name);
      if (typeDef && typeDef.type && typeDef.type.fields) {
        idlFields = typeDef.type.fields;
      }
    }

    const fields: AccountField[] = [];
    for (const idlField of idlFields) {
      const res = calculateIdlTypeSize(idlField.type, typesRegistry);
      fields.push({
        name: idlField.name,
        type: idlTypeToString(idlField.type),
        byteSize: res.byteSize,
        dynamic: res.dynamic,
        ...(res.notes.length > 0 ? { note: res.notes.join("; ") } : {})
      });
    }

    const fieldBytes = fields.reduce((sum, field) => sum + field.byteSize, 0);
    const layoutWarnings = fields
      .filter((field) => field.dynamic)
      .map((field) => ({
        code: "DYNAMIC_TYPE" as const,
        message: "Dynamic size detected. Static realloc analysis may be inaccurate.",
        account: idlAccount.name,
        field: field.name,
        type: field.type,
        filePath: idlPath
      }));

    accounts.push({
      accountId: `${namespace}::${idlAccount.name}`,
      name: idlAccount.name,
      namespace,
      byteSize: 8 + fieldBytes, // 8-byte discriminator
      byteSizeIncludesDiscriminator: true,
      abiFingerprint: idlAbiFingerprint(idlAccount.name, fields),
      hasDynamicSize: layoutWarnings.length > 0,
      layoutWarnings,
      fields,
      filePath: idlPath,
      discriminator: computeAccountDiscriminator(idlAccount.name)
    });
  }

  return accounts;
}

export async function analyzeAnchorIdl(idlPath: string): Promise<AnalyzeResult> {
  const idlContent = await readFile(idlPath, "utf8");
  const idl = JSON.parse(idlContent);
  const accounts = extractAccountsFromIdl(idl, idlPath);
  const instructions: any[] = (idl.instructions || []).map((inst: any) => {
    const name = inst.name;
    const hash = createHash("sha256").update(`global:${name}`).digest();
    const discriminator = "0x" + hash.subarray(0, 8).toString("hex");
    return {
      name,
      discriminator,
      filePath: idlPath
    };
  });
  return {
    projectPath: idlPath,
    accounts: accounts.sort((left, right) => left.name.localeCompare(right.name)),
    instructions
  };
}

function normalizeTypeToString(type: any): string {
  if (typeof type === "string") {
    if (type === "publicKey" || type === "pubkey" || type === "Pubkey") {
      return "publicKey";
    }
    return type;
  }
  if (type && typeof type === "object") {
    if ("defined" in type) {
      if (typeof type.defined === "string") {
        return type.defined;
      }
      if (type.defined && typeof type.defined === "object" && "name" in type.defined) {
        return type.defined.name;
      }
      return JSON.stringify(type.defined);
    }
    if ("option" in type) {
      return `Option<${normalizeTypeToString(type.option)}>`;
    }
    if ("vec" in type) {
      return `Vec<${normalizeTypeToString(type.vec)}>`;
    }
    if ("array" in type && Array.isArray(type.array)) {
      return `[${normalizeTypeToString(type.array[0])}; ${type.array[1]}]`;
    }
  }
  return JSON.stringify(type);
}

export function normalizeIdlToProgramIr(idl: any, filePath = ""): ProgramIr {
  const name = idl.metadata?.name || idl.name || "unknown";
  const version = idl.metadata?.version || idl.version || "0.1.0";
  const idlVersion = idl.metadata?.spec || idl.spec || undefined;
  const programId = idl.metadata?.address || idl.address || undefined;

  const accounts = extractAccountsFromIdl(idl, filePath);

  const instructions: ProgramIrInstruction[] = (idl.instructions || []).map((inst: any) => {
    const instAccounts: ProgramIrAccount[] = (inst.accounts || []).map((acc: any) => {
      return {
        name: acc.name,
        isMut: acc.writable !== undefined ? acc.writable : (acc.isMut !== undefined ? acc.isMut : false),
        isSigner: acc.signer !== undefined ? acc.signer : (acc.isSigner !== undefined ? acc.isSigner : false),
        ...(acc.docs ? { docs: acc.docs } : {}),
        ...(acc.relations ? { relations: acc.relations } : {})
      };
    });

    const args = (inst.args || []).map((arg: any) => ({
      name: arg.name,
      type: normalizeTypeToString(arg.type)
    }));

    return {
      name: inst.name,
      ...(inst.discriminator ? { discriminator: inst.discriminator } : {}),
      accounts: instAccounts,
      args,
      ...(inst.docs ? { docs: inst.docs } : {})
    };
  });

  const types: ProgramIrType[] = (idl.types || []).map((t: any) => {
    const kind = t.type?.kind;
    const fields = t.type?.fields?.map((f: any) => ({
      name: f.name,
      type: normalizeTypeToString(f.type)
    }));
    const variants = t.type?.variants?.map((v: any) => {
      const vFields = v.fields?.map((vf: any) => {
        if (typeof vf === "string" || !("name" in vf)) {
          return { name: "", type: normalizeTypeToString(vf) };
        }
        return {
          name: vf.name,
          type: normalizeTypeToString(vf.type)
        };
      });
      return {
        name: v.name,
        ...(vFields ? { fields: vFields } : {})
      };
    });
    return {
      name: t.name,
      type: {
        kind,
        ...(fields ? { fields } : {}),
        ...(variants ? { variants } : {})
      }
    };
  });

  return {
    name,
    version,
    idlVersion,
    programId,
    accounts,
    instructions,
    types
  };
}

type IDLTypeResolution = {
  byteSize: number;
  dynamic: boolean;
  notes: string[];
};

function calculateIdlTypeSize(
  type: any,
  typesRegistry: Map<string, any>,
  resolving: Set<string> = new Set()
): IDLTypeResolution {
  if (typeof type === "string") {
    switch (type) {
      case "bool":
      case "u8":
      case "i8":
        return { byteSize: 1, dynamic: false, notes: [] };
      case "u16":
      case "i16":
        return { byteSize: 2, dynamic: false, notes: [] };
      case "u32":
      case "i32":
      case "f32":
        return { byteSize: 4, dynamic: false, notes: [] };
      case "u64":
      case "i64":
      case "f64":
        return { byteSize: 8, dynamic: false, notes: [] };
      case "u128":
      case "i128":
        return { byteSize: 16, dynamic: false, notes: [] };
      case "publicKey":
      case "pubkey":
      case "Pubkey":
        return { byteSize: 32, dynamic: false, notes: [] };
      case "bytes":
        return { byteSize: 4, dynamic: true, notes: ["bytes is dynamically sized; counted 4-byte length prefix only"] };
      case "string":
        return { byteSize: 4, dynamic: true, notes: ["string is dynamically sized; counted 4-byte length prefix only"] };
      default:
        if (typesRegistry.has(type)) {
          return resolveDefinedType(type, typesRegistry, resolving);
        }
        return { byteSize: 0, dynamic: true, notes: [`unsupported primitive type: ${type}`] };
    }
  }

  if (type && typeof type === "object") {
    if ("defined" in type) {
      const definedName = typeof type.defined === "string" ? type.defined : type.defined?.name;
      if (typeof definedName === "string") {
        return resolveDefinedType(definedName, typesRegistry, resolving);
      }
    }
    if ("option" in type) {
      const inner = calculateIdlTypeSize(type.option, typesRegistry, resolving);
      return {
        byteSize: 1 + inner.byteSize,
        dynamic: inner.dynamic,
        notes: inner.notes
      };
    }
    if ("vec" in type) {
      const inner = calculateIdlTypeSize(type.vec, typesRegistry, resolving);
      return {
        byteSize: 4, // 4-byte length prefix
        dynamic: true,
        notes: [...inner.notes, `Vec<${idlTypeToString(type.vec)}> is dynamically sized; counted 4-byte length prefix only`]
      };
    }
    if ("array" in type && Array.isArray(type.array) && type.array.length === 2) {
      const innerType = type.array[0];
      const length = Number(type.array[1]);
      const inner = calculateIdlTypeSize(innerType, typesRegistry, resolving);
      return {
        byteSize: inner.byteSize * length,
        dynamic: inner.dynamic,
        notes: inner.notes
      };
    }
  }

  return { byteSize: 0, dynamic: true, notes: [`unknown IDL type structure: ${JSON.stringify(type)}`] };
}

function resolveDefinedType(
  typeName: string,
  typesRegistry: Map<string, any>,
  resolving: Set<string>
): IDLTypeResolution {
  if (resolving.has(typeName)) {
    return { byteSize: 0, dynamic: true, notes: [`circular dependency detected on type: ${typeName}`] };
  }

  const def = typesRegistry.get(typeName);
  if (!def) {
    return { byteSize: 0, dynamic: true, notes: [`defined type not found in IDL: ${typeName}`] };
  }

  resolving.add(typeName);

  let byteSize = 0;
  let dynamic = false;
  const notes: string[] = [];

  if (def.type && def.type.kind === "struct") {
    const fields = def.type.fields || [];
    for (const field of fields) {
      const res = calculateIdlTypeSize(field.type, typesRegistry, resolving);
      byteSize += res.byteSize;
      if (res.dynamic) {
        dynamic = true;
      }
      notes.push(...res.notes);
    }
  } else if (def.type && def.type.kind === "enum") {
    let maxVariantSize = 0;
    const variants = def.type.variants || [];
    const variantSizes: number[] = [];
    for (const variant of variants) {
      let variantSize = 0;
      if (variant.fields) {
        for (const field of variant.fields) {
          const res = calculateIdlTypeSize(field.type, typesRegistry, resolving);
          variantSize += res.byteSize;
          if (res.dynamic) {
            dynamic = true;
          }
          notes.push(...res.notes);
        }
      }
      variantSizes.push(variantSize);
      if (variantSize > maxVariantSize) {
        maxVariantSize = variantSize;
      }
    }
    
    if (variantSizes.length > 1 && new Set(variantSizes).size > 1) {
      dynamic = true;
      notes.push("Enum variants have varying layout sizes, causing tag-based offset shifts at runtime");
    }
    
    byteSize = 1 + maxVariantSize;
  }

  resolving.delete(typeName);

  return { byteSize, dynamic, notes };
}

function idlTypeToString(type: any): string {
  if (typeof type === "string") {
    return type;
  }
  if (type && typeof type === "object") {
    if ("defined" in type) {
      return typeof type.defined === "string" ? type.defined : (type.defined?.name || JSON.stringify(type.defined));
    }
    if ("option" in type) {
      return `Option<${idlTypeToString(type.option)}>`;
    }
    if ("vec" in type) {
      return `Vec<${idlTypeToString(type.vec)}>`;
    }
    if ("array" in type && Array.isArray(type.array)) {
      return `[${idlTypeToString(type.array[0])}; ${type.array[1]}]`;
    }
  }
  return JSON.stringify(type);
}

function idlAbiFingerprint(accountName: string, fields: AccountField[]): string {
  const input = JSON.stringify({
    account: accountName,
    fields: fields.map((field) => ({ name: field.name, type: field.type }))
  });

  return createHash("sha256").update(input).digest("hex");
}

function computeAccountDiscriminator(structName: string): string {
  const hash = createHash("sha256").update(`account:${structName}`).digest();
  return "0x" + hash.subarray(0, 8).toString("hex");
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

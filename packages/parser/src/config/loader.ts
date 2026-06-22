import fs from "node:fs";
import path from "node:path";
import { parse } from "smol-toml";
import type { RawEpicConfig, ResolvedEpicConfig, ResolvedProgram } from "./types.js";
import { EpicConfigSchema } from "./schema.js";
import { validateEpicConfigSecurity } from "./validator.js";

/**
 * Returns default resolved configuration parameters when no epic.toml is found.
 */
export function getDefaultConfig(): ResolvedEpicConfig {
  return {
    compareMode: "ast",
    failOnSeverity: "CRITICAL",
    excludePaths: [],
    enforcePadding: false,
    programs: new Map(),
    ignore: []
  };
}

/**
 * Locates epic.toml by traversing directory upwards from the startDir.
 */
export function findConfigFile(startDir: string): string | null {
  let currentDir = path.resolve(startDir);
  
  // If the startDir is a file, use its directory
  if (fs.existsSync(currentDir) && fs.statSync(currentDir).isFile()) {
    currentDir = path.dirname(currentDir);
  }

  while (true) {
    const candidate = path.join(currentDir, "epic.toml");
    if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) {
      return candidate;
    }
    
    const parent = path.dirname(currentDir);
    if (parent === currentDir) {
      break; // Reached root directory
    }
    currentDir = parent;
  }

  return null;
}

/**
 * Loads, validates, and normalizes epic.toml configuration.
 */
export function loadEpicConfig(configPath?: string): ResolvedEpicConfig {
  const targetPath = configPath ? path.resolve(configPath) : findConfigFile(process.cwd());

  if (!targetPath || !fs.existsSync(targetPath)) {
    return getDefaultConfig();
  }

  try {
    const content = fs.readFileSync(targetPath, "utf-8");
    const rawObject = parse(content);
    
    // Validate structural schema using Zod
    const parsed = EpicConfigSchema.parse(rawObject) as RawEpicConfig;

    // Validate security rules (banned overrides, wildcards, note lengths)
    validateEpicConfigSecurity(parsed);

    const baseDir = path.dirname(targetPath);
    const resolvedPrograms = new Map<string, ResolvedProgram>();

    if (parsed.programs) {
      for (const [name, program] of Object.entries(parsed.programs)) {
        const absoluteProgramPath = path.isAbsolute(program.path)
          ? program.path
          : path.resolve(baseDir, program.path);

        const idlPath = program.idl_path
          ? (path.isAbsolute(program.idl_path) ? program.idl_path : path.resolve(baseDir, program.idl_path))
          : undefined;

        const resolvedOverrides = (program.overrides || []).map(o => ({
          account: o.account,
          finding: o.finding.toUpperCase(),
          field: o.field,
          action: o.action,
          severity: o.severity ? (o.severity.toUpperCase() as any) : undefined,
          note: o.note,
          used: false
        }));

        resolvedPrograms.set(name, {
          name,
          absolutePath: absoluteProgramPath,
          programId: program.id,
          idlPath,
          overrides: resolvedOverrides
        });
      }
    }

    return {
      compareMode: parsed.workspace?.compare_mode || "ast",
      failOnSeverity: (parsed.workspace?.fail_on_severity || "CRITICAL").toUpperCase() as any,
      rpcUrl: parsed.workspace?.rpc_url,
      excludePaths: parsed.workspace?.exclude_paths || [],
      enforcePadding: parsed.workspace?.enforce_padding || false,
      programs: resolvedPrograms,
      ignore: parsed.ignore || []
    };
  } catch (error) {
    if (error instanceof Error) {
      throw new Error(`Failed to load epic.toml: ${error.message}`);
    }
    throw error;
  }
}

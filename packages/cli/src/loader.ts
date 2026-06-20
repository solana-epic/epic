import path from "node:path";
import fs from "node:fs";
import process from "node:process";
import { fileURLToPath } from "node:url";

const PLATFORM_MAP: Record<string, string> = {
  "darwin-arm64": "@epic/cli-darwin-arm64",
  "darwin-x64": "@epic/cli-darwin-x64",
  "linux-x64": "@epic/cli-linux-x64",
  "win32-x64": "@epic/cli-win32-x64"
};

export class EPICBinaryNotFoundError extends Error {
  constructor(platform: string, arch: string, attemptedLocations: string[]) {
    const message = [
      `EPIC Upgrade Guard parser-v2 binary not found.`,
      `Current Platform: ${platform}`,
      `Current Architecture: ${arch}`,
      `Attempted Locations:`,
      ...attemptedLocations.map(loc => `  - ${loc}`)
    ].join("\n");
    super(message);
    this.name = "EPICBinaryNotFoundError";
  }
}

/**
 * Returns the detected platform key (e.g. "darwin-arm64").
 */
export function getPlatformKey(platform = process.platform, arch = process.arch): string {
  return `${platform}-${arch}`;
}

/**
 * Resolves the absolute path to the parser-v2 Rust compiled binary.
 */
export function resolveParserBinary(
  env = process.env,
  platform = process.platform,
  arch = process.arch,
  importMetaUrl = import.meta.url
): string {
  const platformKey = getPlatformKey(platform, arch);
  const packageName = PLATFORM_MAP[platformKey];
  const binName = platform === "win32" ? "parser-v2.exe" : "parser-v2";
  const attempted: string[] = [];

  // 1. Attempt package resolve from dependencies/optionalDependencies
  if (packageName) {
    try {
      const resolvedUrl = import.meta.resolve(packageName);
      const resolvedPath = fileURLToPath(resolvedUrl);
      attempted.push(`NPM Package (${packageName}) -> ${resolvedPath}`);
      if (fs.existsSync(resolvedPath)) {
        return resolvedPath;
      }
    } catch {
      attempted.push(`NPM Package (${packageName}) -> Not installed / Resolution failed`);
    }
  } else {
    attempted.push(`NPM Package -> Unsupported platform key: ${platformKey}`);
  }

  // 2. Attempt local development compilation fallback
  try {
    const __dirname = path.dirname(fileURLToPath(importMetaUrl));
    const localPaths = [
      path.resolve(__dirname, "../../parser-v2/target/release", binName),
      path.resolve(__dirname, "../../parser-v2/target/debug", binName),
      path.resolve(__dirname, "../../../parser-v2/target/release", binName),
      path.resolve(__dirname, "../../../parser-v2/target/debug", binName)
    ];

    for (const localPath of localPaths) {
      attempted.push(`Local Dev Target -> ${localPath}`);
      if (fs.existsSync(localPath)) {
        return localPath;
      }
    }
  } catch {
    attempted.push(`Local Dev Target -> Directory lookup context failed`);
  }

  // 3. Attempt PATH environment traversal lookup
  const pathEnv = env.PATH || "";
  const pathDirs = pathEnv.split(path.delimiter);
  attempted.push(`PATH Lookup -> Scanning ${pathDirs.length} path directories`);
  
  for (const dir of pathDirs) {
    if (!dir) continue;
    const candidatePath = path.join(dir, binName);
    if (fs.existsSync(candidatePath)) {
      try {
        // Simple Windows sanity check (Windows has no X_OK, check exists is sufficient)
        if (platform !== "win32") {
          fs.accessSync(candidatePath, fs.constants.X_OK);
        }
        return candidatePath;
      } catch {
        // Exists but not executable
      }
    }
  }

  throw new EPICBinaryNotFoundError(platform, arch, attempted);
}

import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..");
const artifactsDir = path.join(repoRoot, "artifacts", "local-packages");

// Define package workspaces to pack
const packages = [
  "packages/parser",
  "packages/diff-engine",
  "packages/cli-darwin-arm64",
  "packages/cli-darwin-x64",
  "packages/cli-linux-x64",
  "packages/cli-win32-x64",
  "packages/cli"
];

function runCommand(cmd, cwd = repoRoot) {
  console.log(`Running: ${cmd} (in ${cwd})`);
  execSync(cmd, { cwd, stdio: "inherit" });
}

async function main() {
  try {
    // 1. Build TS workspaces
    console.log("=== Building TypeScript Workspace ===");
    runCommand("npm run build");

    // 2. Prepare artifacts folder
    fs.mkdirSync(artifactsDir, { recursive: true });

    // 3. Stage local compiled Rust binary into the host platform package
    console.log("=== Staging Host Platform Rust Binary ===");
    const binName = process.platform === "win32" ? "parser-v2.exe" : "parser-v2";
    
    // Look for local compiled target binary
    const devBinPaths = [
      path.resolve(repoRoot, "packages/parser-v2/target/release", binName),
      path.resolve(repoRoot, "packages/parser-v2/target/debug", binName),
      path.resolve(repoRoot, "parser-v2/target/release", binName)
    ];

    let sourceBinPath = null;
    for (const p of devBinPaths) {
      if (fs.existsSync(p)) {
        sourceBinPath = p;
        break;
      }
    }

    const hostPlatformPkg = `packages/cli-${process.platform}-${process.arch}`;
    const hostPlatformPkgPath = path.resolve(repoRoot, hostPlatformPkg);

    if (sourceBinPath && fs.existsSync(hostPlatformPkgPath)) {
      const targetBinDir = path.join(hostPlatformPkgPath, "bin");
      fs.mkdirSync(targetBinDir, { recursive: true });
      const targetBinPath = path.join(targetBinDir, binName);
      
      console.log(`Copying compiled binary from: ${sourceBinPath}`);
      console.log(`To target folder: ${targetBinPath}`);
      fs.copyFileSync(sourceBinPath, targetBinPath);
      // Ensure executable permissions on Unix
      if (process.platform !== "win32") {
        fs.chmodSync(targetBinPath, 0o755);
      }
    } else {
      console.warn("⚠️ Warning: No compiled Rust binary found. Staging placeholder files only.");
    }

    // 4. Pack each package workspace
    console.log("=== Packing Workspaces ===");
    for (const pkg of packages) {
      const pkgPath = path.resolve(repoRoot, pkg);
      console.log(`Packing package in: ${pkgPath}`);
      runCommand("npm pack", pkgPath);

      // Find the generated .tgz file and move it to artifacts
      const files = fs.readdirSync(pkgPath);
      const tgzFile = files.find(f => f.endsWith(".tgz"));
      if (tgzFile) {
        const sourcePath = path.join(pkgPath, tgzFile);
        const destPath = path.join(artifactsDir, tgzFile);
        console.log(`Moving ${tgzFile} to artifacts directory...`);
        fs.renameSync(sourcePath, destPath);
      } else {
        throw new Error(`Failed to locate generated tarball in ${pkg}`);
      }
    }

    console.log("\n=== Packaging Completed ===");
    console.log("Artifacts generated in artifacts/local-packages/:");
    const generatedFiles = fs.readdirSync(artifactsDir);
    for (const f of generatedFiles) {
      console.log(`  - ${f}`);
    }
  } catch (error) {
    console.error(`Local packaging failed: ${error.message}`);
    process.exit(1);
  }
}

main();

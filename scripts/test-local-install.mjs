import { execSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..");
const artifactsDir = path.join(repoRoot, "artifacts", "local-packages");

function runCommand(cmd, cwd) {
  console.log(`Running: ${cmd}`);
  try {
    return execSync(cmd, { cwd, stdio: "pipe", encoding: "utf-8" });
  } catch (error) {
    console.error(`Command failed: ${cmd}`);
    console.error(`Stdout: ${error.stdout}`);
    console.error(`Stderr: ${error.stderr}`);
    throw error;
  }
}

async function main() {
  console.log("=== Starting Local Installation Verification ===");

  // 1. Create isolated temporary directory
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "epic-install-test-"));
  console.log(`Created temporary directory: ${tempDir}`);

  try {
    // 2. Identify and locate generated tarballs
    if (!fs.existsSync(artifactsDir)) {
      throw new Error(`Artifacts directory not found: ${artifactsDir}. Please run scripts/package-local.mjs first.`);
    }

    const files = fs.readdirSync(artifactsDir);
    const cliTgz = files.find(f => f.startsWith("epic-cli-0."));
    const parserTgz = files.find(f => f.startsWith("epic-parser-0."));
    const diffEngineTgz = files.find(f => f.startsWith("epic-diff-engine-0."));
    
    // Platform mapping
    const platformKey = `${process.platform}-${process.arch}`;
    const platformTgz = files.find(f => f.startsWith(`epic-cli-${platformKey}-0.`));

    if (!cliTgz || !parserTgz || !diffEngineTgz) {
      throw new Error("Core package tarballs (cli, parser, or diff-engine) are missing from artifacts/local-packages/");
    }

    if (!platformTgz) {
      console.warn(`⚠️ Warning: No pre-matched platform tarball found for current platform (${platformKey}).`);
    }

    const cliPath = path.join(artifactsDir, cliTgz);
    const parserPath = path.join(artifactsDir, parserTgz);
    const diffEnginePath = path.join(artifactsDir, diffEngineTgz);
    
    // 3. Initialize temporary project
    console.log("Initializing dummy package.json...");
    fs.writeFileSync(
      path.join(tempDir, "package.json"),
      JSON.stringify({
        name: "epic-install-test",
        version: "1.0.0",
        type: "module",
        dependencies: {}
      }, null, 2)
    );

    // 4. Install generated tarballs
    console.log("Installing generated tarballs...");
    const installArgs = [parserPath, diffEnginePath, cliPath];
    if (platformTgz) {
      installArgs.push(path.join(artifactsDir, platformTgz));
    }

    const installCmd = `npm install ${installArgs.map(p => `"${p}"`).join(" ")} --force --no-audit --no-fund`;
    console.log(`Executing installation...`);
    const installLog = runCommand(installCmd, tempDir);
    console.log("--- NPM Install Logs ---");
    console.log(installLog);
    console.log("------------------------");

    // 5. Verify optional dependency resolution and loader selection
    console.log("Verifying loader binary resolution...");
    const testLoaderScript = `
import { resolveParserBinary } from "@epic/cli/dist/loader.js";
try {
  const binaryPath = resolveParserBinary();
  console.log("RESOLVED_BINARY_PATH:" + binaryPath);
} catch (err) {
  console.error("LOADER_RESOLUTION_FAILED:" + err.message);
  process.exit(1);
}
`;
    const testScriptPath = path.join(tempDir, "test-loader.js");
    fs.writeFileSync(testScriptPath, testLoaderScript);

    const loaderResult = runCommand("node test-loader.js", tempDir);
    console.log(loaderResult);

    const match = loaderResult.match(/RESOLVED_BINARY_PATH:(.*)/);
    if (!match) {
      throw new Error("Failed to parse binary resolution path from loader stdout.");
    }
    const resolvedBinaryPath = match[1].trim();
    console.log(`✅ Loader successfully resolved binary path to: ${resolvedBinaryPath}`);

    // Verify existence of binary and executability
    if (!fs.existsSync(resolvedBinaryPath)) {
      throw new Error(`Resolved binary path does not exist on disk: ${resolvedBinaryPath}`);
    }
    console.log("✅ Resolved binary exists on disk.");

    // 6. Execute: epic --help
    console.log("Executing 'epic --help'...");
    // We execute it by calling node node_modules/.bin/epic or node node_modules/@epic/cli/dist/index.js
    const helpOutput = runCommand("node node_modules/.bin/epic --help", tempDir);
    console.log("--- epic --help Output ---");
    console.log(helpOutput);
    console.log("-------------------------");
    console.log("✅ 'epic --help' executed successfully.");

    // 7. Execute: epic analyze fixtures/anchor
    console.log("Executing 'epic analyze fixtures/anchor'...");
    const fixturesPath = path.resolve(repoRoot, "fixtures", "anchor");
    const analyzeOutput = runCommand(`node node_modules/.bin/epic analyze "${fixturesPath}"`, tempDir);
    console.log("--- epic analyze Output ---");
    console.log(analyzeOutput);
    console.log("---------------------------");

    // Verify output structure
    if (!analyzeOutput.includes("Analyzing Solana Program Workspace") || !analyzeOutput.includes("STATE ACCOUNTS:")) {
      throw new Error("epic analyze output did not contain expected headers/analysis results.");
    }
    console.log("✅ 'epic analyze' executed and output validated successfully.");

    console.log("\n🎉 ALL LOCAL INSTALLATION TESTS PASSED SUCCESSFULLY!");
    console.log(`Temporary directory: ${tempDir}`);
  } catch (error) {
    console.error(`\n❌ Local installation verification failed: ${error.message}`);
    process.exit(1);
  } finally {
    // Cleanup temporary directory
    try {
      console.log(`Cleaning up temporary directory: ${tempDir}`);
      fs.rmSync(tempDir, { recursive: true, force: true });
    } catch (cleanupErr) {
      console.warn(`Failed to clean up temporary directory ${tempDir}: ${cleanupErr.message}`);
    }
  }
}

main();

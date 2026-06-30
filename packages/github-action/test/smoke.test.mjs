import test from "node:test";
import assert from "node:assert";
import { execFileSync } from "node:child_process";
import * as path from "node:path";
import * as fs from "node:fs";
import * as os from "node:os";

const ACTION_DIR = path.resolve(import.meta.dirname, "..");
const ENTRY_POINT = path.resolve(ACTION_DIR, "dist", "index.js");

// Paths to the fixtures used for integration testing
const FIXTURES_DIR = path.resolve(ACTION_DIR, "..", "..", "examples", "compatibility-demo");
const COMPATIBLE_OLD = path.join(FIXTURES_DIR, "01-compatible", "old");
const COMPATIBLE_NEW = path.join(FIXTURES_DIR, "01-compatible", "new");
const BLOCKED_OLD = path.join(FIXTURES_DIR, "03-blocked", "old");
const BLOCKED_NEW = path.join(FIXTURES_DIR, "03-blocked", "new");

test("GitHub Action Smoke Tests", async (t) => {
  // Ensure the entrypoint exists
  assert.ok(fs.existsSync(ENTRY_POINT), "Action entrypoint dist/index.js must exist before running tests. Run 'npm run build' first.");

  // Helper to run the action with specific paths
  const runAction = (oldPath, newPath) => {
    // Create temporary files for GitHub Action outputs
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "epic-action-test-"));
    const summaryFile = path.join(tmpDir, "summary.md");
    const outputFile = path.join(tmpDir, "output.md");
    
    fs.writeFileSync(summaryFile, "");
    fs.writeFileSync(outputFile, "");

    try {
      const stdout = execFileSync(process.execPath, [ENTRY_POINT], {
        env: {
          ...process.env,
          GITHUB_STEP_SUMMARY: summaryFile,
          GITHUB_OUTPUT: outputFile,
          INPUT_GITHUB_TOKEN: "mock-token",
          INPUT_OLD_PATH: oldPath,
          INPUT_NEW_PATH: newPath,
          // We don't set GITHUB_EVENT_PATH so it skips PR-specific API calls
        },
        encoding: "utf8",
        cwd: tmpDir // Run in tmp dir to isolate the sarif output
      });

      return {
        status: 0,
        stdout,
        summary: fs.readFileSync(summaryFile, "utf8"),
        output: fs.readFileSync(outputFile, "utf8")
      };
    } catch (err) {
      if (err.status) {
        return {
          status: err.status,
          stdout: err.stdout,
          stderr: err.stderr,
          summary: fs.readFileSync(summaryFile, "utf8"),
          output: fs.readFileSync(outputFile, "utf8")
        };
      }
      throw err; // Unexpected error (e.g. process couldn't start)
    } finally {
      // Cleanup
      fs.rmSync(tmpDir, { recursive: true, force: true });
    }
  };

  await t.test("executes successfully on a compatible upgrade (exit 0)", () => {
    const result = runAction(COMPATIBLE_OLD, COMPATIBLE_NEW);
    
    assert.strictEqual(result.status, 0, "Expected action to exit with 0 for a compatible upgrade");
    assert.match(result.stdout, /EPIC Guard approved upgrade/, "Expected approval message in stdout");
    
    // Verify markdown summary was generated without throwing
    assert.ok(result.summary.length > 0, "Expected a Markdown summary to be written to GITHUB_STEP_SUMMARY");
    assert.match(result.summary, /Compatible/, "Expected 'Compatible' to be in the Markdown summary");
    
    // Verify outputs were set
    assert.match(result.output, /severity<<[\s\S]*Compatible/, "Expected severity output to be set to Compatible");
  });

  await t.test("fails and correctly propagates exit code on a blocked upgrade (exit non-zero)", () => {
    const result = runAction(BLOCKED_OLD, BLOCKED_NEW);
    
    assert.notStrictEqual(result.status, 0, "Expected action to exit with non-zero for a blocked upgrade");
    assert.match(result.stdout, /EPIC Guard Blocked: deploying would corrupt existing on-chain accounts/, "Expected blocked message in stdout");
    
    // Verify markdown summary was still generated
    assert.ok(result.summary.length > 0, "Expected a Markdown summary to be written to GITHUB_STEP_SUMMARY even on failure");
    assert.match(result.summary, /Blocked/, "Expected 'Blocked' to be in the Markdown summary");
    
    // Verify outputs were set
    assert.match(result.output, /severity<<[\s\S]*Blocked/, "Expected severity output to be set to Blocked");
  });
});

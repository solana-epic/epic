import assert from "node:assert/strict";
import { test } from "node:test";
import path from "node:path";
import { getPlatformKey, resolveParserBinary, EPICBinaryNotFoundError } from "../dist/loader.js";

test("loader: getPlatformKey maps platforms and architectures", () => {
  assert.equal(getPlatformKey("darwin", "arm64"), "darwin-arm64");
  assert.equal(getPlatformKey("linux", "x64"), "linux-x64");
  assert.equal(getPlatformKey("win32", "x64"), "win32-x64");
});

test("loader: resolves darwin-arm64 binary placeholder from package exports", () => {
  const binaryPath = resolveParserBinary({}, "darwin", "arm64");
  assert.ok(binaryPath.endsWith("packages/cli-darwin-arm64/bin/parser-v2"));
});

test("loader: resolves darwin-x64 binary placeholder from package exports", () => {
  const binaryPath = resolveParserBinary({}, "darwin", "x64");
  assert.ok(binaryPath.endsWith("packages/cli-darwin-x64/bin/parser-v2"));
});

test("loader: resolves linux-x64 binary placeholder from package exports", () => {
  const binaryPath = resolveParserBinary({}, "linux", "x64");
  assert.ok(binaryPath.endsWith("packages/cli-linux-x64/bin/parser-v2"));
});

test("loader: resolves win32-x64 binary placeholder from package exports", () => {
  const binaryPath = resolveParserBinary({}, "win32", "x64");
  assert.ok(binaryPath.endsWith("packages/cli-win32-x64/bin/parser-v2.exe"));
});

test("loader: throws EPICBinaryNotFoundError when all discovery options fail", () => {
  const badEnv = { PATH: "" };
  
  // Point importMetaUrl to a context that fails local dev check
  const badImportMetaUrl = "file:///nonexistent/subfolder/src/loader.js";

  assert.throws(() => {
    resolveParserBinary(badEnv, "freebsd", "x64", badImportMetaUrl);
  }, (err) => {
    assert.ok(err instanceof EPICBinaryNotFoundError);
    assert.match(err.message, /EPIC Upgrade Guard parser-v2 binary not found/);
    assert.match(err.message, /Current Platform: freebsd/);
    assert.match(err.message, /Current Architecture: x64/);
    assert.match(err.message, /Attempted Locations:/);
    assert.match(err.message, /Unsupported platform key: freebsd-x64/);
    assert.match(err.message, /Local Dev Target -> /);
    assert.match(err.message, /PATH Lookup -> /);
    return true;
  });
});

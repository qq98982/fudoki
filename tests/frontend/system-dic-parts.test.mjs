import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const repoRoot = resolve(__dirname, "../..");
const partsDir = resolve(repoRoot, "resources/system.dic.parts");
const manifestPath = resolve(partsDir, "manifest.json");
const maxGitHubBlobBytes = 100 * 1024 * 1024;

test("system dictionary parts reconstruct the expected dictionary bytes", () => {
  assert.equal(existsSync(manifestPath), true, "manifest.json must exist");

  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  assert.equal(manifest.file, "resources/system.dic");
  assert.equal(typeof manifest.size, "number");
  assert.match(manifest.sha256, /^[0-9a-f]{64}$/);
  assert.ok(Array.isArray(manifest.parts));
  assert.ok(manifest.parts.length > 0);

  const hash = createHash("sha256");
  let totalSize = 0;

  for (const partName of manifest.parts) {
    assert.match(partName, /^system\.dic\.part\.\d{3}$/);

    const partPath = resolve(partsDir, partName);
    assert.equal(existsSync(partPath), true, `${partName} must exist`);

    const stats = statSync(partPath);
    assert.ok(stats.size > 0, `${partName} must not be empty`);
    assert.ok(stats.size < maxGitHubBlobBytes, `${partName} must be smaller than 100 MiB`);

    const bytes = readFileSync(partPath);
    totalSize += bytes.length;
    hash.update(bytes);
  }

  assert.equal(totalSize, manifest.size);
  assert.equal(hash.digest("hex"), manifest.sha256);
});

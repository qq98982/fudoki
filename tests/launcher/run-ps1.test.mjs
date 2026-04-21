import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

test("run.ps1 pulls git lfs assets and starts Rust app", () => {
  const source = readFileSync("run.ps1", "utf8");

  assert.match(source, /resources[\\/]+system\.dic/);
  assert.match(source, /git lfs pull/);
  assert.match(source, /cargo build/);
  assert.match(source, /cargo run/);
  assert.doesNotMatch(source, /api\.github\.com\/repos\/WorksApplications\/SudachiDict\/releases\/latest/);
});

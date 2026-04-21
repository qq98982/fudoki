import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

test("run.ps1 downloads latest Sudachi core dictionary and starts Rust app", () => {
  const source = readFileSync("run.ps1", "utf8");

  assert.match(source, /resources[\\/]+system\.dic/);
  assert.match(
    source,
    /api\.github\.com\/repos\/WorksApplications\/SudachiDict\/releases\/latest/,
  );
  assert.match(source, /Invoke-RestMethod/);
  assert.match(source, /Expand-Archive/);
  assert.match(source, /cargo build/);
  assert.match(source, /cargo run/);
});

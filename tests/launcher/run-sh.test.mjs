import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

test("run.sh downloads latest Sudachi core dictionary and starts Rust app", () => {
  const source = readFileSync("run.sh", "utf8");

  assert.match(source, /resources\/system\.dic/);
  assert.match(
    source,
    /https:\/\/api\.github\.com\/repos\/WorksApplications\/SudachiDict\/releases\/latest/,
  );
  assert.match(source, /core\.zip/);
  assert.match(source, /cargo build/);
  assert.match(source, /cargo run/);
});

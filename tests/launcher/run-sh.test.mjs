import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

test("run.sh pulls git lfs assets and starts Rust app", () => {
  const source = readFileSync("run.sh", "utf8");

  assert.match(source, /resources\/system\.dic/);
  assert.match(source, /git lfs pull/);
  assert.match(source, /cargo build/);
  assert.match(source, /\.env/);
  assert.match(source, /dotenv_bind_addr/);
  assert.match(source, /127\.0\.0\.1:8000/);
  assert.match(source, /http:\/\/\$bind_addr/);
  assert.match(source, /FUDOKI_BIND_ADDR.*0\.0\.0\.0:8000/);
  assert.match(source, /cargo run/);
  assert.doesNotMatch(source, /api\.github\.com\/repos\/WorksApplications\/SudachiDict\/releases\/latest/);
});

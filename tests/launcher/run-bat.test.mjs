import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

test("run.bat delegates to run.ps1", () => {
  const source = readFileSync("run.bat", "utf8");

  assert.match(source, /powershell/i);
  assert.match(source, /ExecutionPolicy Bypass/i);
  assert.match(source, /run\.ps1/i);
});

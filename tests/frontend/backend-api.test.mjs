import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  analyzeTextRequest,
  lookupDictionaryRequest,
  resolveTtsText,
  waitForBackendReady,
} from "../../static/js/backend-api.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const indexHtmlSource = readFileSync(resolve(__dirname, "../../index.html"), "utf8");
const mainJsSource = readFileSync(resolve(__dirname, "../../static/main-js.js"), "utf8");
const ttsJsSource = readFileSync(resolve(__dirname, "../../static/js/tts.js"), "utf8");

test("backend api exports expected helpers", () => {
  assert.equal(typeof resolveTtsText, "function");
  assert.equal(typeof waitForBackendReady, "function");
  assert.equal(typeof analyzeTextRequest, "function");
  assert.equal(typeof lookupDictionaryRequest, "function");
});

test("resolveTtsText prefers tts_text over reading", () => {
  assert.equal(resolveTtsText({ reading: "ブラウザー", tts_text: "browser" }), "browser");
});

test("resolveTtsText falls back to reading when tts_text is missing", () => {
  assert.equal(resolveTtsText({ reading: "リアクト", surface: "React" }), "リアクト");
});

test("resolveTtsText falls back to surface when reading and tts_text are empty", () => {
  assert.equal(resolveTtsText({ reading: "", tts_text: "", surface: "browser" }), "browser");
});

test("main-js analyzes and looks up via backend helper APIs", () => {
  assert.ok(mainJsSource.includes("window.FudokiBackendApi.analyzeTextRequest"));
  assert.ok(mainJsSource.includes("window.FudokiBackendApi.lookupDictionaryRequest"));
});

test("index.html skips Firebase auth redirect on localhost", () => {
  assert.ok(indexHtmlSource.includes("window.location.hostname"));
  assert.ok(indexHtmlSource.includes("localhost"));
  assert.ok(indexHtmlSource.includes("skipping Firebase auth redirect"));
});

test("main-js only auto-analyzes when structure signature changes", () => {
  assert.ok(mainJsSource.includes("let lastInputStructureSignature = computeStructureSignature(textInput ? textInput.value : '');"));
  assert.ok(mainJsSource.includes("let lastAnalyzedStructureSignature = lastInputStructureSignature;"));
  assert.ok(mainJsSource.includes("const currentSig = computeStructureSignature(textInput.value);"));
  assert.ok(mainJsSource.includes("if (currentSig === lastInputStructureSignature) {"));
  assert.ok(mainJsSource.includes("lastAnalyzedStructureSignature = currentSig;"));
  assert.ok(!mainJsSource.includes("let inputAnalyzeTimeout = null;"));
  assert.ok(!mainJsSource.includes("inputAnalyzeTimeout = setTimeout(() => {"));
});

test("main-js includes remote tts cache metadata in speak payloads", () => {
  assert.ok(mainJsSource.includes("payload.document_id"));
  assert.ok(mainJsSource.includes("payload.document_revision"));
  assert.ok(mainJsSource.includes("payload.cache_scope_version"));
});

test("main-js bootstraps analysis only after backend readiness", () => {
  assert.ok(mainJsSource.includes("async function waitForBackendApiClient("));
  assert.ok(mainJsSource.includes("async function bootstrapAnalysis()"));
  assert.ok(mainJsSource.includes("const backendApi = await waitForBackendApiClient();"));
  assert.ok(mainJsSource.includes("backendApi.waitForBackendReady()"));
  assert.ok(mainJsSource.includes("bootstrapAnalysis();"));
});

test("tts playback resolves speech text via backend helper", () => {
  assert.ok(ttsJsSource.includes("window.FudokiBackendApi.resolveTtsText"));
});

test("waitForBackendReady retries until health endpoint returns ready", async () => {
  let calls = 0;
  const fetcher = async () => {
    calls += 1;
    if (calls < 3) {
      return { ok: true, json: async () => ({ status: "starting" }) };
    }
    return {
      ok: true,
      json: async () => ({ status: "ready", tokenizer_ready: true, dictionary_ready: true }),
    };
  };

  const result = await waitForBackendReady({ fetcher, retries: 3, delayMs: 0 });
  assert.equal(result.status, "ready");
  assert.equal(calls, 3);
});

test("analyzeTextRequest posts text payload and returns JSON", async () => {
  let request = null;
  const fetcher = async (url, init) => {
    request = { url, init };
    return { ok: true, json: async () => ({ lines: [] }) };
  };

  const result = await analyzeTextRequest("こんにちは", fetcher);
  assert.deepEqual(result, { lines: [] });
  assert.equal(request.url, "/api/analyze");
  assert.equal(request.init.method, "POST");
  assert.equal(request.init.headers["Content-Type"], "application/json");
  assert.equal(request.init.body, JSON.stringify({ text: "こんにちは" }));
});

test("lookupDictionaryRequest returns null on 404", async () => {
  const fetcher = async () => ({ status: 404, ok: false, json: async () => ({}) });
  const result = await lookupDictionaryRequest("未登録", fetcher);
  assert.equal(result, null);
});

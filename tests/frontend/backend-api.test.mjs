import test from "node:test";
import assert from "node:assert/strict";

import {
  analyzeTextRequest,
  lookupDictionaryRequest,
  resolveTtsText,
  waitForBackendReady,
} from "../../static/js/backend-api.js";

test("backend api exports expected helpers", () => {
  assert.equal(typeof resolveTtsText, "function");
  assert.equal(typeof waitForBackendReady, "function");
  assert.equal(typeof analyzeTextRequest, "function");
  assert.equal(typeof lookupDictionaryRequest, "function");
});

test("resolveTtsText prefers tts_text over reading", () => {
  assert.equal(resolveTtsText({ reading: "ブラウザー", tts_text: "browser" }), "browser");
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

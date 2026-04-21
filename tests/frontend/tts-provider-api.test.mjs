import test from "node:test";
import assert from "node:assert/strict";

import {
  fetchTtsProviders,
  requestRemoteSpeech,
} from "../../static/js/backend-api.js";

test("backend api exports tts helpers", () => {
  assert.equal(typeof fetchTtsProviders, "function");
  assert.equal(typeof requestRemoteSpeech, "function");
});

test("fetchTtsProviders reads provider metadata from the backend", async () => {
  let requestedUrl = null;
  const fetcher = async (url) => {
    requestedUrl = url;
    return {
      ok: true,
      json: async () => ({ default_provider: "system", providers: [{ id: "system" }] }),
    };
  };

  const result = await fetchTtsProviders(fetcher);
  assert.equal(requestedUrl, "/api/tts/providers");
  assert.equal(result.default_provider, "system");
  assert.equal(result.providers[0].id, "system");
});

test("requestRemoteSpeech posts provider payload and returns the response", async () => {
  let request = null;
  const response = {
    ok: true,
    headers: {
      get: (key) => {
        const normalized = String(key || "").toLowerCase();
        if (normalized === "content-type") return "audio/mpeg";
        if (normalized === "x-fudoki-tts-cache") return "hit";
        return null;
      },
    },
    arrayBuffer: async () => new Uint8Array([1, 2, 3]).buffer,
  };
  const fetcher = async (url, init) => {
    request = { url, init };
    return response;
  };

  const result = await requestRemoteSpeech(
    { provider: "openai-compatible", text: "こんにちは", voice: "alloy", format: "mp3", speed: 1.0 },
    fetcher,
  );

  assert.equal(result, response);
  assert.equal(result.headers.get("x-fudoki-tts-cache"), "hit");
  assert.equal(request.url, "/api/tts/speak");
  assert.equal(request.init.method, "POST");
  assert.equal(request.init.headers["Content-Type"], "application/json");
  assert.equal(
    request.init.body,
    JSON.stringify({
      provider: "openai-compatible",
      text: "こんにちは",
      voice: "alloy",
      format: "mp3",
      speed: 1.0,
    }),
  );
});

test("requestRemoteSpeech surfaces backend error messages", async () => {
  const fetcher = async () => ({
    ok: false,
    json: async () => ({
      error: {
        code: "tts_request_failed",
        message: "TTS request failed: 401 Unauthorized",
      },
    }),
  });

  await assert.rejects(
    () => requestRemoteSpeech({ provider: "openai-compatible", text: "こんにちは" }, fetcher),
    /TTS request failed: 401 Unauthorized/,
  );
});

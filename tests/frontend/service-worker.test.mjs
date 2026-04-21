import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const serviceWorkerSource = readFileSync(resolve(__dirname, "../../service-worker.js"), "utf8");

test("service worker bumps cache version and refreshes static assets in background", () => {
  assert.ok(serviceWorkerSource.includes("const CACHE_VERSION = 'v2';"));
  assert.ok(serviceWorkerSource.includes("event.respondWith(staleWhileRevalidate(req, event));"));
  assert.ok(serviceWorkerSource.includes("async function staleWhileRevalidate(request, event) {"));
  assert.ok(serviceWorkerSource.includes("event.waitUntil(refreshCachedResponse(request));"));
});

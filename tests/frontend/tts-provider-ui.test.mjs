import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const mainJsSource = readFileSync(resolve(__dirname, "../../static/main-js.js"), "utf8");
const i18nSource = readFileSync(resolve(__dirname, "../../static/js/i18n.js"), "utf8");
const stylesSource = readFileSync(resolve(__dirname, "../../static/styles.css"), "utf8");

test("settings markup includes a provider selector and provider status slot", () => {
  assert.ok(mainJsSource.includes("ttsProviderSelect"));
  assert.ok(mainJsSource.includes("ttsProviderStatus"));
});

test("main-js loads provider metadata from backend helpers", () => {
  assert.ok(mainJsSource.includes("window.FudokiBackendApi.fetchTtsProviders"));
  assert.ok(mainJsSource.includes("window.FudokiBackendApi.requestRemoteSpeech"));
  assert.ok(mainJsSource.includes("window.FudokiRemoteTts.createRemoteTtsPlayer"));
});

test("provider state is stored in localStorage and defaults from backend metadata", () => {
  assert.ok(mainJsSource.includes("ttsProvider: 'ttsProvider'"));
  assert.ok(mainJsSource.includes("default_provider"));
});

test("system provider option is always available regardless of backend metadata shape", () => {
  assert.ok(mainJsSource.includes("SYSTEM_TTS_PROVIDER_ID"));
  assert.ok(mainJsSource.includes("Always include system provider"));
});

test("i18n includes labels for the new provider UI", () => {
  assert.ok(i18nSource.includes("ttsProviderLabel"));
  assert.ok(i18nSource.includes("ttsProviderSystem"));
  assert.ok(i18nSource.includes("ttsProviderRemote"));
  assert.ok(i18nSource.includes("ttsStatusAvailable"));
  assert.ok(i18nSource.includes("ttsStatusRequestFailed"));
});

test("styles include a compact provider status treatment", () => {
  assert.ok(stylesSource.includes(".tts-provider-status"));
  assert.ok(stylesSource.includes(".tts-provider-status.is-error"));
});

import test from "node:test";
import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const readmeSource = readFileSync(resolve(__dirname, "../../README.md"), "utf8");
const gitignoreSource = readFileSync(resolve(__dirname, "../../.gitignore"), "utf8");
const envExamplePath = resolve(__dirname, "../../.env.example");
const mainJsSource = readFileSync(resolve(__dirname, "../../static/main-js.js"), "utf8");
const i18nSource = readFileSync(resolve(__dirname, "../../static/js/i18n.js"), "utf8");
const stylesSource = readFileSync(resolve(__dirname, "../../static/styles.css"), "utf8");

test("settings markup includes a provider selector and provider status slot", () => {
  assert.ok(mainJsSource.includes("ttsProviderSelect"));
  assert.ok(mainJsSource.includes("ttsProviderStatus"));
  assert.ok(mainJsSource.includes("remoteTtsModelSelect"));
  assert.ok(mainJsSource.includes("remoteTtsVoiceSelect"));
});

test("main-js loads provider metadata from backend helpers", () => {
  assert.ok(mainJsSource.includes("window.FudokiBackendApi.fetchTtsProviders"));
  assert.ok(mainJsSource.includes("window.FudokiBackendApi.requestRemoteSpeech"));
  assert.ok(mainJsSource.includes("window.FudokiRemoteTts.createRemoteTtsPlayer"));
});

test("provider state is stored in localStorage and defaults from backend metadata", () => {
  assert.ok(mainJsSource.includes("ttsProvider: 'ttsProvider'"));
  assert.ok(mainJsSource.includes("ttsRemoteModel: 'ttsRemoteModel'"));
  assert.ok(mainJsSource.includes("ttsRemoteVoice: 'ttsRemoteVoice'"));
  assert.ok(mainJsSource.includes("default_provider"));
  assert.ok(mainJsSource.includes("provider.options.models"));
  assert.ok(mainJsSource.includes("provider.options.voices"));
});

test("system provider option is always available regardless of backend metadata shape", () => {
  assert.ok(mainJsSource.includes("SYSTEM_TTS_PROVIDER_ID"));
  assert.ok(mainJsSource.includes("Always include system provider"));
});

test("i18n includes labels for the new provider UI", () => {
  assert.ok(i18nSource.includes("ttsProviderLabel"));
  assert.ok(i18nSource.includes("ttsProviderSystem"));
  assert.ok(i18nSource.includes("ttsProviderRemote"));
  assert.ok(i18nSource.includes("remoteTtsModelLabel"));
  assert.ok(i18nSource.includes("remoteTtsVoiceLabel"));
  assert.ok(i18nSource.includes("ttsStatusAvailable"));
  assert.ok(i18nSource.includes("ttsStatusRequestFailed"));
});

test("styles include a compact provider status treatment", () => {
  assert.ok(stylesSource.includes(".tts-provider-status"));
  assert.ok(stylesSource.includes(".tts-provider-status.is-error"));
});

test("playback entry points delegate line and full text through the selected provider", () => {
  assert.ok(mainJsSource.includes("playTextThroughSelectedProvider(lineText, 'line')"));
  assert.ok(mainJsSource.includes("playTextThroughSelectedProvider(readingParts, 'full')"));
  assert.ok(mainJsSource.includes("playTextThroughSelectedProvider(text, 'full')"));
  assert.ok(mainJsSource.includes("playTextThroughSelectedProvider(textToSpeak, 'token')"));
  assert.ok(mainJsSource.includes("payload.model"));
  assert.ok(mainJsSource.includes("payload.voice"));
  assert.ok(mainJsSource.includes("payload.speed"));
});

test(".env.example is present and documents the required remote tts keys", () => {
  assert.equal(existsSync(envExamplePath), true);
  const envExampleSource = readFileSync(envExamplePath, "utf8");
  assert.ok(envExampleSource.includes("FUDOKI_TTS_OPENAI_BASE_URL"));
  assert.ok(envExampleSource.includes("FUDOKI_TTS_OPENAI_API_KEY"));
  assert.ok(envExampleSource.includes("FUDOKI_TTS_OPENAI_MODEL"));
  assert.ok(envExampleSource.includes("FUDOKI_TTS_OPENAI_MODEL_OPTIONS"));
  assert.ok(envExampleSource.includes("FUDOKI_TTS_OPENAI_VOICE_OPTIONS"));
});

test("gitignore excludes the local .env file", () => {
  assert.ok(gitignoreSource.includes(".env"));
});

test("README explains online tts configuration and env defaults", () => {
  assert.ok(readmeSource.includes("FUDOKI_TTS_OPENAI_BASE_URL"));
  assert.ok(readmeSource.includes("FUDOKI_TTS_DEFAULT_PROVIDER"));
  assert.ok(readmeSource.includes("OpenAI-compatible"));
  assert.ok(readmeSource.includes("FUDOKI_TTS_OPENAI_MODEL_OPTIONS"));
  assert.ok(readmeSource.includes("FUDOKI_TTS_OPENAI_VOICE_OPTIONS"));
});

# Fudoki Remote TTS Options Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add env-driven remote model and voice option lists to the OpenAI-compatible TTS path, expose them in the UI, and send the selected values with remote playback while reusing the existing shared speed control.

**Architecture:** Extend the backend TTS provider metadata to return remote defaults plus allowed `models` and `voices`, sourced from `.env` option lists or safe built-in fallbacks. Keep the frontend source of truth in backend metadata, persist user model/voice choices only when they remain valid, and continue using the existing single speed control for both system and remote playback.

**Tech Stack:** Rust, Cargo, axum, serde, Node built-in test runner, existing static frontend JS

---

## Planned File Structure

**Modify**

- `src/tts.rs`
- `tests/tts_providers_api.rs`
- `tests/tts_speak_api.rs`
- `static/main-js.js`
- `static/js/i18n.js`
- `tests/frontend/tts-provider-ui.test.mjs`
- `.env.example`
- `.env`
- `README.md`

## Task 1: Extend Backend Provider Metadata With Model/Voice Options

**Files:**
- Modify: `src/tts.rs`
- Modify: `tests/tts_providers_api.rs`

- [ ] **Step 1: Write failing provider metadata tests for remote model/voice options**
- [ ] **Step 2: Run `cargo test --test tts_providers_api` and verify failure**
- [ ] **Step 3: Implement backend parsing for `FUDOKI_TTS_OPENAI_MODEL_OPTIONS` and `FUDOKI_TTS_OPENAI_VOICE_OPTIONS`, add defaults/options to `/api/tts/providers`, and validate saved/default selection fallbacks**
- [ ] **Step 4: Re-run `cargo test --test tts_providers_api` and verify pass**
- [ ] **Step 5: Commit**

## Task 2: Validate Requested Remote Model/Voice In Speak Endpoint

**Files:**
- Modify: `src/tts.rs`
- Modify: `tests/tts_speak_api.rs`

- [ ] **Step 1: Write failing speak-endpoint tests for invalid model/voice rejection and selected model forwarding**
- [ ] **Step 2: Run `cargo test --test tts_speak_api` and verify failure**
- [ ] **Step 3: Implement request `model` support plus allowed-list validation for remote model/voice**
- [ ] **Step 4: Re-run `cargo test --test tts_speak_api` and verify pass**
- [ ] **Step 5: Commit**

## Task 3: Add Remote Model/Voice UI And Persisted Selection

**Files:**
- Modify: `static/main-js.js`
- Modify: `static/js/i18n.js`
- Modify: `tests/frontend/tts-provider-ui.test.mjs`

- [ ] **Step 1: Write failing frontend tests for remote model/voice controls and payload inclusion**
- [ ] **Step 2: Run `node --test tests/frontend/tts-provider-ui.test.mjs` and verify failure**
- [ ] **Step 3: Implement remote model/voice selects, persisted selection with validity fallback, and remote payload inclusion while keeping token playback on system**
- [ ] **Step 4: Re-run `node --test tests/frontend/tts-provider-ui.test.mjs` and verify pass**
- [ ] **Step 5: Commit**

## Task 4: Update Env Examples And Documentation

**Files:**
- Modify: `.env.example`
- Modify: `.env`
- Modify: `README.md`

- [ ] **Step 1: Add env option-list examples and update docs for remote model/voice selection**
- [ ] **Step 2: Run the full verification suite: `cargo test` and `node --test tests/frontend/*.test.mjs`**
- [ ] **Step 3: Commit**

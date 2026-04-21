# Analysis Refresh and Remote TTS Cache Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the 250ms auto-analysis refresh loop and add process-local remote TTS caching with document-scoped and global-setting invalidation.

**Architecture:** Keep the analysis trigger logic in the existing frontend flow, but gate automatic refreshes strictly on structure signature changes. Extend the remote TTS request contract with document/cache metadata and add an in-memory cache in `src/tts.rs` that reuses audio bytes, expires old entries, and clears on document or remote-setting changes.

**Tech Stack:** Rust, axum, reqwest, Node test runner, browser-side JavaScript

---

### Task 1: Rework Frontend Analysis Triggers

**Files:**
- Modify: `static/main-js.js`
- Test: `tests/frontend/backend-api.test.mjs`

- [ ] **Step 1: Write the failing frontend assertions**

```js
test("main-js only auto-analyzes when structure signature changes", () => {
  assert.ok(mainJsSource.includes("let lastAnalyzedStructureSignature ="));
  assert.ok(mainJsSource.includes("const currentSig = computeStructureSignature(textInput.value);"));
  assert.ok(mainJsSource.includes("if (currentSig === lastInputStructureSignature) {"));
  assert.ok(!mainJsSource.includes("inputAnalyzeTimeout = setTimeout(() => {"));
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test tests/frontend/backend-api.test.mjs`
Expected: FAIL because `static/main-js.js` still contains the debounced input analysis path.

- [ ] **Step 3: Write the minimal frontend implementation**

```js
let lastInputStructureSignature = computeStructureSignature(textInput ? textInput.value : '');
let lastAnalyzedStructureSignature = lastInputStructureSignature;

textInput.addEventListener('input', () => {
  const currentSig = computeStructureSignature(textInput.value);
  if (currentSig === lastInputStructureSignature) {
    return;
  }
  lastInputStructureSignature = currentSig;
  analyzeText();
});
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test tests/frontend/backend-api.test.mjs`
Expected: PASS

### Task 2: Extend Remote TTS Request Metadata

**Files:**
- Modify: `static/main-js.js`
- Modify: `src/tts.rs`
- Test: `tests/tts_speak_api.rs`

- [ ] **Step 1: Write the failing API tests**

```rust
#[tokio::test]
async fn speak_endpoint_reuses_cached_audio_for_same_document_revision() { /* ... */ }

#[tokio::test]
async fn speak_endpoint_evicts_only_changed_document_revision() { /* ... */ }
```

```js
assert.ok(mainJsSource.includes("payload.document_id"));
assert.ok(mainJsSource.includes("payload.document_revision"));
assert.ok(mainJsSource.includes("payload.cache_scope_version"));
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test tts_speak_api -- --nocapture`
Expected: FAIL because `SpeakRequest` does not yet carry document/cache metadata.

Run: `node --test tests/frontend/backend-api.test.mjs`
Expected: FAIL because the frontend does not yet send the new metadata fields.

- [ ] **Step 3: Write the minimal request contract implementation**

```rust
pub struct SpeakRequest {
    pub provider: Option<String>,
    pub text: String,
    pub model: Option<String>,
    pub voice: Option<String>,
    pub format: Option<String>,
    pub speed: Option<f32>,
    pub document_id: Option<String>,
    pub document_revision: Option<u64>,
    pub cache_scope_version: Option<String>,
}
```

```js
payload.document_id = activeDocId;
payload.document_revision = activeDocRevision;
payload.cache_scope_version = currentRemoteTtsCacheScopeVersion();
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test tts_speak_api -- --nocapture`
Expected: cache-metadata tests compile and pass

Run: `node --test tests/frontend/backend-api.test.mjs`
Expected: PASS

### Task 3: Add Backend Remote TTS Cache and Invalidation

**Files:**
- Modify: `src/tts.rs`
- Test: `tests/tts_speak_api.rs`

- [ ] **Step 1: Write the failing backend cache tests**

```rust
#[tokio::test]
async fn speak_endpoint_reuses_cached_audio_for_same_effective_request() { /* assert upstream called once */ }

#[tokio::test]
async fn speak_endpoint_clears_entire_cache_when_cache_scope_changes() { /* assert second document refetches */ }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test tts_speak_api -- --nocapture`
Expected: FAIL because there is no cache store or invalidation logic.

- [ ] **Step 3: Write the minimal cache implementation**

```rust
struct CacheKey { /* document_id, text, model, voice, speed, format */ }
struct CacheEntry { bytes: Vec<u8>, content_type: String, expires_at: Instant, last_used: u64 }
struct TtsCache { entries: HashMap<CacheKey, CacheEntry>, document_versions: HashMap<String, u64>, cache_scope_version: Option<String> }
```

```rust
if cache_scope_changed { cache.clear_all(); }
if document_revision_changed { cache.clear_document(document_id); }
if let Some(hit) = cache.get(&key) { return Ok(hit); }
let upstream = fetch_upstream().await?;
cache.insert(key, upstream.clone());
```

- [ ] **Step 4: Run targeted tests to verify they pass**

Run: `cargo test tts_speak_api -- --nocapture`
Expected: PASS

- [ ] **Step 5: Run full project verification**

Run: `npm test`
Expected: all Rust and frontend tests pass

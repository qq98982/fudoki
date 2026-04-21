# Fudoki Analysis Refresh and Remote TTS Cache Design

Date: 2026-04-22

## Summary

Fudoki will reduce unnecessary UI churn during editing by removing the current 250ms auto-analysis loop and only re-running automatic analysis when document structure changes. Manual analysis, initial document load, and document switching will continue to trigger analysis.

Fudoki will also add an in-memory backend cache for remote OpenAI-compatible TTS audio. The cache will be process-local only, will not write audio to disk, and will reuse synthesized audio when the effective remote TTS request is identical. Cache reuse will cover the user case of switching away from a document and returning to it without changing text or remote TTS settings. If remote OpenAI-related settings change, the whole remote TTS cache will be cleared. If a document changes, only cache entries associated with that document will be cleared.

## Context

The current frontend re-analyzes text shortly after almost every input event. That causes visible loading-state churn even when the user is only making small inline edits that do not materially change line or sentence structure.

The current remote TTS path also performs a fresh upstream request on every playback. The backend requests audio from the configured OpenAI-compatible `/audio/speech` endpoint, returns the bytes to the browser, and the browser converts those bytes into a temporary object URL for playback. There is no persistence on disk and there is no cache reuse across repeated identical requests.

The desired user-visible behavior is:

- editing should not repeatedly refresh the analysis pane when nothing structurally important changed
- switching to another document and back should not trigger a new remote TTS request when the text and remote TTS options are unchanged
- remote TTS audio must stay in memory only and must not be written as local files

## Goals

- Remove the current 250ms auto-analysis behavior that fires during ordinary typing.
- Keep automatic analysis when the document structure changes.
- Keep manual analysis available at all times.
- Keep analysis on document load and document switch so the visible result stays aligned with the active document.
- Add process-local backend caching for remote TTS audio.
- Reuse cached audio when the effective remote TTS request is identical.
- Avoid writing generated audio to disk.
- Bound memory usage for cached audio.
- Clear the whole remote TTS cache when global remote TTS settings change.
- Clear only the changed document's remote TTS entries when document content changes.

## Non-Goals

- Building a persistent on-disk audio cache.
- Caching browser system TTS output.
- Adding a frontend Blob cache in the first version.
- Performing global cache flushes whenever any document changes.
- Redesigning the existing analysis UI.

## Chosen Approach

### Analysis Refresh

Automatic analysis will be driven only by a structure signature derived from the current text. The app will no longer trigger analysis after a short fixed debounce on every `input` event.

The structure signature will remain lightweight and deterministic. The first implementation will continue to use a signature derived from:

- non-empty line count
- sentence boundary count

This keeps the current concept but changes how it is used. The signature becomes the gate for automatic analysis, not just a secondary blur-time check.

### Remote TTS Cache

Remote TTS audio will be cached in the Rust backend using a process-local in-memory cache with:

- exact request-key matching
- document-scoped invalidation
- global-setting invalidation
- TTL-based expiration
- LRU-style capacity eviction

This approach is sufficient for the user workflow where the service remains running and the user revisits previously played content within the same session.

## Alternatives Considered

### Option A: Keep Auto-Analysis but Increase Debounce

This would reduce request frequency but would still cause analysis pane churn during normal typing. It does not match the requested behavior closely enough.

### Option B: Disable Auto-Analysis Entirely

This would eliminate churn but would also make structural edits feel stale unless the user manually clicks analyze every time. The user explicitly wanted structure changes to continue refreshing automatically.

### Option C: Frontend Audio Cache

This could avoid some repeated requests, but it complicates lifecycle management, memory control, and replay behavior across navigation and playback state changes. Backend caching is simpler and more reliable for the first pass.

## Analysis Refresh Design

### Current Problems

The current input handler:

- starts a 250ms timer after most edits
- runs analysis even during ordinary word-level edits
- shows loading and result replacement repeatedly while the user is still typing

That behavior makes the page feel busy even when the user has not done anything that warrants a visible refresh.

### New Trigger Rules

The system will analyze in these cases:

- user clicks the analyze button
- active document is loaded at startup
- user switches to another document
- current text changes in a way that changes the structure signature

The system will not analyze automatically in these cases:

- ordinary text edits that keep the same structure signature
- blur events where the structure signature is unchanged
- autosave-only updates

### Expected User Experience

- editing a word within the same sentence should not refresh the right pane
- adding or removing a line should refresh the right pane
- splitting or joining sentences should refresh the right pane
- switching documents should still refresh the right pane to show the active document analysis

### Implementation Notes

Frontend changes will stay local to the current analysis flow:

- remove the 250ms delayed input-triggered analysis path
- keep tracking the previous structure signature
- on `input`, recompute the current signature and trigger analysis only when it differs from the previous stored signature
- on `blur`, stop treating blur itself as a reason to analyze; only analyze if the signature has changed since the last analyzed state
- after a successful auto-analysis or manual analysis, update the stored signature baseline

The document autosave behavior remains unchanged.

## Remote TTS Cache Design

### Cache Scope

The cache will exist only inside the current Rust backend process.

Properties:

- no audio files are written to disk
- restarting the backend clears all cached audio
- browser refresh does not matter as long as the backend process stays alive

### Cache Key

The cache key must represent the effective upstream speech request. The first version will key on:

- document identifier
- provider
- text
- model
- voice
- speed
- format

This key handles reuse for unchanged document content. The cache will also maintain separate invalidation state so that:

- changing a document clears all entries previously associated with that document
- changing global remote TTS settings clears the entire remote TTS cache

### Cache Value

Each cache entry will store:

- audio bytes
- content type
- insertion or last-access timestamp needed for TTL/LRU management

### Invalidation Model

The cache will support both document-scoped invalidation and whole-cache invalidation.

Behavior:

- if document content changes, clear all cache entries associated with that document
- if provider changes, clear the entire remote TTS cache
- if model changes, clear the entire remote TTS cache
- if voice changes, clear the entire remote TTS cache
- if format changes, clear the entire remote TTS cache
- if backend OpenAI-compatible configuration changes, the process restart already clears the entire cache
- if speed changes while all global settings stay the same, cache behavior remains request-key-based and different speeds do not reuse each other

This satisfies the user requirement that changed content should only invalidate the changed article, while changed OpenAI-related settings should invalidate all previously synthesized remote audio.

### Document Association

To support document-scoped invalidation, the remote TTS request will include enough document identity to group cache entries by article.

The first version will add optional fields to the remote TTS request:

- `document_id`
- `document_revision`

Rules:

- when remote TTS is requested from an active saved document, frontend sends both fields
- when the backend sees the same `document_id` with a different `document_revision`, it evicts all cache entries associated with that document before continuing
- if no document identity is available, the backend falls back to plain request-key caching without document-scoped eviction

### Expiration and Capacity

To prevent unbounded memory growth, the cache will apply both:

- TTL: default 30 minutes
- capacity limit: default 64 entries

When an entry is older than TTL, it is treated as expired and removed on access. When capacity is exceeded, the least recently used entry is evicted.

These values are implementation defaults and can be adjusted later if memory measurements justify it.

### Request Flow

1. Frontend requests `POST /api/tts/speak` with remote TTS options and, when available, document identity fields.
2. Backend checks whether the document revision changed and evicts that document's old entries if needed.
3. Backend checks whether global remote TTS settings were changed by the frontend and clears the cache if needed.
4. Backend computes a cache key from the effective request parameters.
5. If a non-expired entry exists, backend returns cached audio immediately.
6. Otherwise backend requests upstream audio, stores the result in cache, and returns it.

## Backend Architecture Changes

### `src/tts.rs`

Add a small cache layer inside the existing TTS module:

- cache key type
- cache entry type
- process-local shared cache store
- document-to-cache-entry index
- global cache clear path for remote setting changes
- TTL check
- LRU eviction bookkeeping

The cache should wrap only successful upstream responses. Failed requests must not be cached.

### `src/app.rs`

No new endpoint is required for the first version. Existing `POST /api/tts/speak` will gain cache-backed behavior internally and accept optional document identity and cache-invalidation metadata.

## Frontend Changes

### `static/main-js.js`

Analysis flow changes:

- stop scheduling unconditional delayed analysis after input
- trigger auto-analysis only when structure signature changes
- avoid blur-time unconditional re-analysis

Remote TTS flow changes:

- include active document identity when requesting remote TTS
- send cache-clear intent when remote provider/model/voice/format changes
- keep playback flow otherwise unchanged because the backend will decide whether a response comes from cache or upstream

## Error Handling

### Analysis

- if a structure-triggered analysis fails, keep existing error-state rendering
- ordinary typing without structural change should not enter loading state because no analysis request should be made

### Remote TTS

- upstream failures continue to surface as playback errors
- cache lookup failure or eviction must silently fall through to normal upstream fetch
- failed upstream requests must not poison future cache hits

## Testing

### Frontend Tests

Add or update tests to verify:

- input without structure change does not trigger analysis
- input with structure change does trigger analysis
- document switch still triggers analysis
- manual analyze button still triggers analysis

### Backend Tests

Add tests to verify:

- identical remote TTS requests hit cache after the first success
- changed document revision evicts only that document's cached entries
- changed provider, model, or voice clears the whole cache
- expired entries are not reused
- cache capacity eviction removes least recently used entries
- failed upstream requests are not cached

## Risks

- The current structure signature is intentionally coarse. Some text edits that change tokenization but not line/sentence structure will no longer auto-refresh until manual analyze is used.
- Audio bytes can be large. Capacity and TTL must remain conservative to avoid excessive memory usage.
- LRU bookkeeping must stay simple and correct under concurrent requests.

## Rollout

This change can ship without a migration step.

- existing documents and local settings remain valid
- no new user configuration is required
- cache behavior activates automatically when remote TTS is used

## Recommendation

Proceed with:

- structure-signature-gated auto-analysis
- process-local remote TTS cache keyed by effective request parameters
- 30 minute TTL
- 64 entry LRU capacity

This is the smallest change set that directly addresses the current sluggishness and the repeated remote TTS generation cost without introducing file persistence or frontend cache complexity.

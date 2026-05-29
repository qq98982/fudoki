# Fudoki Codebase Audit — Implementation Status & Improvement Opportunities

Date: 2026-05-15

## 1. Architecture Overview

Fudoki is a local-first Japanese text analysis + TTS web app: **React 19 + TypeScript frontend**, **Rust (Axum) backend**, **SQLite persistence**.

### 1.1 Current State: What's Implemented

| Layer | Status | Key Files |
|-------|--------|-----------|
| Backend HTTP router | ✅ Complete | [src/app.rs](src/app.rs:1) |
| Sudachi tokenizer | ✅ Complete | [src/analyzer.rs](src/analyzer.rs:1) |
| English token handler | ✅ Basic (5 overrides) | [src/english.rs](src/english.rs:1) |
| Numeric reading generation | ✅ Complete | [src/number_reading.rs](src/number_reading.rs:1) |
| Particle reading normalization | ✅ Complete | [src/reading_normalization.rs](src/reading_normalization.rs:1) |
| JMdict dictionary lookup | ✅ Complete | [src/dictionary.rs](src/dictionary.rs:1) |
| Remote TTS (OpenAI-compatible) | ✅ Complete + dual cache | [src/tts.rs](src/tts.rs:1) |
| SQLite persistence (WAL, FK, busy timeout) | ✅ Complete | [src/storage/db.rs](src/storage/db.rs:1) |
| Document CRUD with revision-based concurrency | ✅ Complete | [src/storage/documents_repo.rs](src/storage/documents_repo.rs:1) |
| Settings key-value store | ✅ Complete | [src/storage/settings_repo.rs](src/storage/settings_repo.rs:1) |
| Analysis cache (by content hash + revision) | ✅ Complete | [src/storage/analysis_cache_repo.rs](src/storage/analysis_cache_repo.rs:1) |
| TTS persistent audio cache | ✅ Complete | [src/storage/tts_cache_repo.rs](src/storage/tts_cache_repo.rs:1) |
| TTS in-memory cache (30min TTL, LRU) | ✅ Complete | [src/tts.rs](src/tts.rs:106) |
| Legacy browser-data migration | ✅ Complete | [src/storage/documents_repo.rs](src/storage/documents_repo.rs:140) |
| Analysis cache clear endpoint | ✅ Complete | [src/app.rs](src/app.rs:104) |
| React frontend shell | ✅ Complete | [frontend/src/App.tsx](frontend/src/App.tsx:1) |
| Document rail (list, search, CRUD) | ✅ Complete | [frontend/src/features/documents/DocumentRail.tsx](frontend/src/features/documents/DocumentRail.tsx:1) |
| Inline analysis strip (token display) | ✅ Complete | [frontend/src/features/analysis/AnalysisStrip.tsx](frontend/src/features/analysis/AnalysisStrip.tsx:1) |
| Inspector panel (4 tabs) | ✅ Complete | [frontend/src/features/inspector/InspectorPanel.tsx](frontend/src/features/inspector/InspectorPanel.tsx:1) |
| Trilingual i18n (EN/JA/ZH) | ✅ Complete | [frontend/src/lib/i18n.ts](frontend/src/lib/i18n.ts:1) |
| Typed API client | ✅ Complete | [frontend/src/lib/api.ts](frontend/src/lib/api.ts:1) |
| System TTS adapter | ✅ Complete | [frontend/src/lib/systemTts.ts](frontend/src/lib/systemTts.ts:1) |
| Remote TTS text segmentation | ✅ Complete | [frontend/src/lib/ttsSegmentation.ts](frontend/src/lib/ttsSegmentation.ts:1) |
| Zustand store (shell state) | ✅ Complete | [frontend/src/app/store.ts](frontend/src/app/store.ts:1) |
| Rust test suite (12 test binaries) | ✅ Compiles | `cargo test --no-run` succeeds |
| Frontend test suite | ✅ Present | `frontend/src/**/*.test.tsx` |

### 1.2 Design Specs vs Implementation

Reference design documents (in priority order):

| Design Doc | Key Commitments |
|------------|-----------------|
| [Sudachi Backend Design](docs/superpowers/specs/2026-04-21-sudachi-backend-design.md) | English 4-tier handler, TTS `tts_text` contract, health check startup flow |
| [React Frontend Rewrite Design](docs/superpowers/specs/2026-04-22-react-frontend-rewrite-design.md) | CodeMirror 6 editor, TanStack Query for server state, no service worker, structure-signature analysis gating |
| [Analysis Refresh & TTS Cache Design](docs/superpowers/specs/2026-04-22-analysis-refresh-and-remote-tts-cache-design.md) | Structure-signature-gated auto-analysis, TTS cache with document-scoped invalidation |

---

## 2. Gaps: Design vs Implementation

### 2.1 🔴 HIGH: Editor is `<textarea>`, not CodeMirror 6

**Design commitment**: The React rewrite design explicitly chose CodeMirror 6 as the editor engine ([spec](docs/superpowers/specs/2026-04-22-react-frontend-rewrite-design.md:159)):

> CodeMirror 6 as the initial editor engine in plain-text mode  
> CodeMirror 6 is chosen because it is a better long-term boundary than a raw `textarea` or EasyMDE

**Current state**: `@uiw/react-codemirror` is listed in [frontend/package.json](frontend/package.json:15) as a dependency **but is never imported anywhere in `frontend/src/`** (search_content `@uiw/react-codemirror` across frontend/src returns 0 matches). The editor is a plain `<textarea>` in [EditorPane.tsx](frontend/src/features/editor/EditorPane.tsx:38).

**Impact**: No selection-aware features, no syntax highlighting, no upgrade path to Markdown mode. The CodeMirror dependency is dead weight (≈200KB in node_modules).

### 2.2 🔴 HIGH: Auto-analysis uses text equality, not structure signatures

**Design commitment**: The analysis refresh design ([spec](docs/superpowers/specs/2026-04-22-analysis-refresh-and-remote-tts-cache-design.md:48)):

> Automatic analysis will be driven only by a structure signature derived from the current text.

The legacy `static/main-js.js` already implements `computeStructureSignature()` with proper gating (search_content `computeStructureSignature` on static/main-js.js confirms it). The legacy backend-api test ([tests/frontend/backend-api.test.mjs:50](tests/frontend/backend-api.test.mjs:50)) explicitly verifies this behavior.

**Current state**: The React frontend uses simple text equality — `draft === lastAnalyzedTextRef.current` — at [App.tsx:273](frontend/src/App.tsx:273). This means ANY edit triggers re-analysis after the 1200ms debounce, even adding a single character to the same line.

**Impact**: Unnecessary API calls on every keystroke that doesn't change line/sentence structure. The legacy frontend already solved this — the React rewrite regressed.

### 2.3 🟡 MEDIUM: No `/api/health` startup readiness check in frontend

**Design commitment** ([spec](docs/superpowers/specs/2026-04-21-sudachi-backend-design.md:213)):

> On page startup, call `/api/health`. If the backend is not ready, show a dedicated non-fatal startup state.

**Current state**: The frontend calls `listDocuments()` directly on mount — no health check precedes it. Search for `/api/health` in `frontend/src/` returns 0 matches. The health endpoint exists at [src/app.rs:53](src/app.rs:53) but nothing consumes it.

### 2.4 🟡 MEDIUM: `cache_scope_version` never sent by frontend

The backend supports cache-scope-based TTS cache invalidation via `SpeakRequest.cache_scope_version` ([src/tts.rs:33](src/tts.rs:33)), and the persistent cache repo handles scope changes ([src/storage/tts_cache_repo.rs:75](src/storage/tts_cache_repo.rs:75)). But the React frontend never sends this field — search_content `cache_scope_version` in `frontend/src/` returns 0 matches.

**Impact**: When remote TTS settings change (provider/model/voice), the in-memory cache isn't proactively cleared by the frontend — it relies on the TTL and LRU eviction only. The design intended frontend-driven invalidation.

### 2.5 🟡 MEDIUM: English token handler missing Tier 3 (dictionary-corrected)

**Design commitment** ([spec](docs/superpowers/specs/2026-04-21-sudachi-backend-design.md:145)) — 4-tier classification:

1. Abbreviation ✅
2. Built-in override ✅ (5 entries only)
3. **Dictionary-corrected English term** ❌ NOT IMPLEMENTED
4. Unknown English term ✅

**Current state**: The `classify_english_token` function in [src/english.rs](src/english.rs:38) has only 5 hardcoded overrides. Any English word not in that list and not all-uppercase falls through to "unknown" with `confidence: 0`. There is no dictionary-backed tier.

**Impact**: Common loanwords like `server`, `browser`, `vintage`, `client`, `internet` are treated as unknown English (no reading, tts_text = original). The design explicitly called for a dictionary-corrected tier that would provide reasonable Japanese-style readings.

### 2.6 🟢 LOW: Missing document duplicate endpoint

Design mentioned `POST /api/documents/:id/duplicate` as "optional but useful" ([spec](docs/superpowers/specs/2026-04-22-react-frontend-rewrite-design.md:255)). Not implemented — search_content `duplicate` in `src/` returns 0 matches.

### 2.7 🟢 LOW: Analysis cache has no size limit

The `analysis_cache` table in SQLite grows unboundedly — no TTL, no max entries, no eviction policy. Compare with TTS cache which has both 30min TTL and 64-entry LRU cap ([src/storage/tts_cache_repo.rs:19](src/storage/tts_cache_repo.rs:19)). The only way to clear analysis cache is the manual "Clear analysis cache" button.

### 2.8 🟢 LOW: Romaji converter only handles katakana, not hiragana

The `kanaMap` in [analysisTokens.ts](frontend/src/features/analysis/analysisTokens.ts:7) maps only katakana → romaji. Sudachi returns hiragana readings, but `tokenRomaji()` would produce raw hiragana characters for those. The katakana conversion works because `reading_normalization.rs` outputs katakana, but if any path produces hiragana, romaji display breaks silently.

### 2.9 🟢 LOW: No SPA fallback for client-side routing

The backend serves `/` → `index.html` and `/login.html` explicitly ([src/app.rs:62](src/app.rs:62)), but has no catch-all route. If the React app later adds client-side routing (e.g., `/documents/:id`), direct navigation would 404. The current code only has a one-level fallback `PathBuf::from("index.html")` for when `frontend/dist/index.html` is missing.

### 2.10 🟢 LOW: Service worker file still exists

`service-worker.js` exists at repo root but is not registered in the React frontend (search_content `service-worker` in `frontend/src/` returns 0). The design explicitly called for removal. The file is orphaned — should be deleted to avoid confusion.

---

## 3. Optimization Opportunities (Not in Design Docs)

### 3.1 Add hiragana to the romaji map

The `kanaMap` in [analysisTokens.ts](frontend/src/features/analysis/analysisTokens.ts:7) should also include hiragana mappings for robustness. Even though Sudachi outputs katakana readings normally, edge cases (particles, some readings) could produce hiragana.

### 3.2 Backend graceful shutdown

[src/main.rs](src/main.rs:1) has no signal handling (SIGTERM/SIGINT). Adding `tokio::signal` would allow clean connection pool teardown.

### 3.3 Analysis cache eviction policy

Add TTL (e.g., 7 days) and/or max row count to `analysis_cache` table, similar to the TTS cache. Could be a simple `DELETE FROM analysis_cache WHERE last_used_at < ?` on insert.

### 3.4 Reduce CodeMirror dead dependency

Either implement CodeMirror (aligns with design) or remove `@uiw/react-codemirror` from `package.json` to reduce install size. Current state is worst of both worlds: dependency installed but unused.

### 3.5 Frontend `any` types in settings

[App.tsx:108](frontend/src/App.tsx:108) uses `typeof settings.ttsProvider === 'string'` — the settings are typed as `Record<string, unknown>`. A typed settings schema would catch mismatches at compile time.

### 3.6 Empty draft saves on document create

`handleCreateDocument` ([App.tsx:203](frontend/src/App.tsx:203)) calls `persistDraft()` before creating a new document, which can trigger a save of empty content if the previous document was modified. Should only save if there's actual unsaved content.

### 3.7 Document list re-sort on every mutation

`updateDocumentsCache` ([App.tsx:594](frontend/src/App.tsx:594)) re-sorts all documents on every update. For large document lists, this is O(n log n) per save. A position-maintaining insertion would be O(n).

---

## 4. Summary: Priority Matrix

| Priority | Item | Effort | Impact |
|----------|------|--------|--------|
| **P0** | Replace `<textarea>` with CodeMirror 6 | Medium | Design alignment, Markdown path, UX |
| **P0** | Implement structure-signature-gated auto-analysis | Small | Reduce unnecessary API calls by ~80% |
| **P1** | Add frontend `/api/health` startup check | Small | Better error UX, design alignment |
| **P1** | Implement dictionary-corrected English tier | Medium | Fixes "unknown English" for common loanwords |
| **P1** | Send `cache_scope_version` from frontend | Small | TTS cache invalidation on settings change |
| **P2** | Add analysis cache eviction policy | Small | Prevent unbounded DB growth |
| **P2** | Add hiragana to romaji kanaMap | Small | Robustness |
| **P3** | Add document duplicate endpoint | Small | UX convenience |
| **P3** | Remove orphaned `service-worker.js` | Trivial | Cleanup |
| **P3** | Add SPA catch-all route | Trivial | Future-proofing |
| **P3** | Add backend graceful shutdown | Small | Production readiness |
| **P3** | Type settings schema | Small | Type safety |

---

## 5. Next Steps Recommendation

**Phase A (this iteration)**: P0 items — CodeMirror 6 integration + structure-signature gating. These are the two biggest gaps between design and implementation.

**Phase B (next iteration)**: P1 items — health check, English dictionary tier, cache scope version.

**Phase C (maintenance)**: P2/P3 items — eviction policies, cleanup, hardening.

# Fudoki Rust Sudachi Backend Design

Date: 2026-04-21

## Summary

Fudoki will move Japanese analysis from an in-browser tokenizer to a local Rust backend service. The backend will serve the existing frontend static assets and expose analysis and dictionary APIs. The primary analysis engine will be `sudachi.rs` with a local Sudachi dictionary on disk. This change is intended to improve tokenization quality, English and loanword handling, runtime reliability, and long-term local-app maintainability.

## Context

The current application is a static frontend that performs tokenization and reading generation in the browser with `kuromoji` and `kuroshiro`. This causes several user-visible issues:

- Ordinary English words are misclassified as abbreviations and pronounced letter-by-letter.
- Tokenization quality is limited for loanwords, product names, and modern vocabulary.
- Dictionary lookup requires loading large client-side data and scanning it linearly.
- Initialization failure silently degrades to character-level fallback tokenization.
- Analysis feels button-driven or blur-driven instead of like a local app.

The user is comfortable starting a local service before opening the frontend, and prefers the implementation to be Rust-based rather than Python-based.

## Goals

- Improve segmentation quality by replacing browser tokenization with `sudachi.rs`.
- Fix English token handling so ordinary English words are not treated as abbreviations.
- Preserve correct behavior for real abbreviations such as `AI`, `API`, and `CPU`.
- Keep the current frontend UI and workflow largely intact.
- Make dictionary lookup local and fast enough for interactive token-card usage.
- Remove silent quality degradation when the analysis engine fails to initialize.
- Implement the backend as a Rust local service.

## Non-Goals

- Rebuilding the frontend framework or redesigning the UI.
- Shipping a cloud-hosted service.
- Supporting arbitrary browser-only offline execution in this phase.
- Solving perfect English-to-katakana conversion for unknown words.
- Converting the app into a desktop GUI shell in this phase.

## Chosen Approach

The system will become a local web application with one Rust backend process:

- `axum` serves `index.html` and `/static/*`.
- `axum` also exposes `/api/*` endpoints for analysis and dictionary lookup.
- `sudachi.rs` becomes the primary tokenizer.
- Frontend analysis logic is switched from local `JapaneseSegmenter` calls to HTTP API calls.
- Existing browser TTS, document management, settings, and reading mode remain in the frontend.

This keeps the current UI but moves the quality-critical language logic into a local Rust runtime.

## Alternatives Considered

### Option A: Keep pure frontend and improve dictionaries

This would reduce some lookup issues but would not fix the main source of bad English and loanword analysis. Tokenization quality would still be limited by the browser-side stack.

### Option B: Local Python backend with SudachiPy

This would be the fastest path, but it is no longer the selected direction because the user explicitly prefers the Rust implementation.

### Option C: Local Rust backend with `axum + sudachi.rs`

This is the chosen option. It improves the core analysis engine, aligns with the user's implementation preference, and fits a local-tool architecture better than the current browser tokenizer.

## Architecture

### Backend

Create a Rust crate with focused modules:

- `src/main.rs`
  - process startup
  - configuration loading
  - server bootstrap
- `src/app.rs`
  - router construction
  - shared state wiring
- `src/models.rs`
  - request and response structs
  - serialization contracts
- `src/analyzer.rs`
  - Sudachi tokenizer wrapper
  - line-based analysis entrypoint
  - reading and POS normalization
- `src/english.rs`
  - English token classifier and correction pipeline
- `src/dictionary.rs`
  - local dictionary loading
  - query and lookup helpers

### Frontend

Keep the current frontend files, but change the data source:

- `static/main-js.js`
  - `analyzeText()` will call `POST /api/analyze`
  - token-card translation loading will call `GET /api/dictionary`
- `static/js/tts.js`
  - token playback will prefer backend-provided `tts_text`
- existing rendering functions continue to consume token arrays with minimal structural change

## Dictionary Runtime Strategy

### Sudachi Dictionary

The first Rust implementation will use a local Sudachi dictionary file on disk rather than trying to bake the dictionary into the binary.

Rationale:

- it keeps the first migration simpler and more reliable
- it avoids a custom build pipeline just to embed dictionary assets
- the current `sudachi.rs` project documentation still treats dictionary baking as not implemented

This means the first release target is “single local service plus local dictionary files”, not “single self-contained binary”.

### Lookup Dictionary

JMdict-derived local data remains the main source for token-card meaning lookup.

## Data Flow

### Analyze Flow

1. User edits text in the current frontend.
2. Frontend sends `POST /api/analyze` with raw text.
3. Backend splits input by lines, normalizes whitespace conservatively, and runs Sudachi tokenization.
4. Backend applies English token handling rules after tokenization.
5. Backend returns normalized token data grouped by line.
6. Frontend renders tokens with the current UI pipeline.

### Dictionary Flow

1. User clicks a token card.
2. Frontend requests `GET /api/dictionary?term=...`.
3. Backend resolves the best lookup term and returns structured dictionary info.
4. Frontend renders the result in the existing detail panel.

## English Token Handling

This is the main behavior change and uses an explicit precedence order.

### Guiding Principle

When the system has high confidence, English-origin terms should be read the way a Japanese speaker would commonly say them in Japanese usage, not by naive letter-by-letter spelling.

Examples:

- `React` -> `リアクト`
- `Docker` -> `ドッカー`
- `GitHub` -> `ギットハブ`
- `API` -> `エーピーアイ`

When the system does not have high confidence, it must avoid inventing a Japanese-style reading.

### Classification Order

1. Abbreviation
2. Built-in override
3. Dictionary-corrected English term
4. Unknown English term

### Abbreviation Rule

Treat as abbreviation only when the token is a high-confidence abbreviation, such as:

- all-uppercase alphabetic token like `AI`, `CPU`, `API`
- explicitly whitelisted mixed-case abbreviations if needed later

Abbreviations keep Japanese letter-name pronunciation behavior.

### Built-in Override Rule

Use a curated built-in map for common technical words and product names where the application wants stable, predictable readings, such as:

- `React`
- `Docker`
- `TypeScript`
- `JavaScript`

### Dictionary-Corrected English Rule

If a token is alphabetic but not an abbreviation, query local correction data. If the backend can confidently map the English token to the Japanese reading or expression that is commonly used in practice, use that reading and mark its source as dictionary-corrected.

### Unknown English Rule

If no reliable correction exists, preserve the original English token and do not fabricate katakana. This avoids the current failure mode where ordinary English words are pronounced letter-by-letter.

### TTS Contract for Unknown English

Unknown English terms must not rely on the display-oriented `reading` field for speech behavior.

- `reading` may be empty when the system intentionally avoids inventing a Japanese reading.
- `tts_text` must be returned for every token.
- for unknown English terms, `tts_text` must default to the original token text
- for abbreviations, overrides, and dictionary-corrected terms, `tts_text` should normally match the resolved pronunciation text

This keeps display behavior and speech behavior separate and avoids regressions where the UI looks correct but TTS becomes undefined or inconsistent.

## Dictionary Sources

### Primary Analysis Dictionary

- local Sudachi dictionary files compatible with `sudachi.rs`

### Primary Lookup Dictionary

- JMdict-derived local data

### English Correction Data

The first implementation should support an offline local correction source that can be built into project data. Acceptable inputs include:

- user-provided local dictionary data
- official downloadable dictionary sources such as EDRDG JMdict
- supplementary Japanese dictionary data suitable for English-to-Japanese correction

The runtime system should not depend on parsing `MDX` directly in the frontend. If an `MDX` file is used, it must be converted offline into a project-local queryable format first.

## API Contract

### `GET /api/health`

Purpose:

- backend availability check
- frontend startup sanity check

Response:

- `status`
- tokenizer readiness
- dictionary readiness

### `POST /api/analyze`

Request:

```json
{
  "text": "..."
}
```

Response:

```json
{
  "lines": [
    [
      {
        "surface": "React",
        "lemma": "React",
        "reading": "リアクト",
        "tts_text": "リアクト",
        "pos": ["名詞"],
        "source": "override",
        "confidence": 1.0
      }
    ]
  ]
}
```

Required token fields:

- `surface`
- `lemma`
- `reading`
- `tts_text`
- `pos`
- `source`
- `confidence`

Token field semantics:

- `reading`
  - display-oriented reading shown in the token UI
  - may be empty for unknown English terms
- `tts_text`
  - speech-oriented text used by token, line, and full-text playback
  - always present
  - for unknown English terms, defaults to the original token text

### `GET /api/dictionary?term=...`

Response fields must support the current token-card UI:

- `word`
- `query`
- `kanji`
- `kana`
- `senses`
- `hasMultipleMeanings`
- `totalResults`
- `lookupSource`

Minimum response shape:

```json
{
  "word": "React",
  "query": "リアクト",
  "kanji": [],
  "kana": [
    { "text": "リアクト", "common": false }
  ],
  "senses": [
    {
      "gloss": "React",
      "partOfSpeech": ["noun"],
      "field": [],
      "misc": [],
      "info": [],
      "chineseSource": null
    }
  ],
  "hasMultipleMeanings": false,
  "totalResults": 1,
  "lookupSource": "jmdict"
}
```

Each `sense` must preserve these fields because the current frontend detail modal depends on them:

- `gloss`
- `partOfSpeech`
- `field`
- `misc`
- `info`
- `chineseSource`

## Error Handling

- If Sudachi initialization fails, backend startup should fail visibly instead of silently degrading into character tokenization.
- If dictionary data is unavailable, `/api/dictionary` should return a structured error or empty result, not hang.
- Frontend should show a user-visible failure state when `/api/analyze` cannot be reached.
- Frontend should not attempt browser-side tokenization fallback in this phase.

## Frontend Integration Changes

### Immediate Changes

- Replace `initSegmenter()` and browser `segment()` usage with a backend API client.
- Move translation lookup from `window.dictionaryService` to backend requests.
- Update playback code to prefer `token.tts_text` over `token.reading`.
- Keep existing token rendering and TTS UI behavior.

### Backend Readiness on Startup

The frontend must not immediately call `/api/analyze` on first load without confirming backend readiness.

Required startup behavior:

1. On page startup, call `/api/health`.
2. If the backend is not ready, show a dedicated non-fatal startup state such as "backend starting" instead of a generic analysis failure.
3. Retry `/api/health` with bounded backoff.
4. Only trigger automatic initial analysis after `/api/health` reports ready.
5. If readiness is not reached within the retry window, keep the page usable and allow manual retry.

This prevents regressions where existing auto-analysis runs before the local backend is ready.

### Transitional Compatibility

The old browser tokenization libraries may remain temporarily in the repo during migration, but they are no longer the primary analysis path. They should not be used as a silent fallback.

## Performance Expectations

- Analysis should feel local and immediate on typical note-sized input.
- Dictionary lookup should be fast enough for token-card interaction without loading the full dataset into browser memory.
- The Rust service should remain lightweight enough for normal local startup without a complex dependency chain.

## Migration Plan

### Phase 1

- add Rust backend serving static assets and health endpoint
- wire frontend to backend connectivity checks

### Phase 2

- add Sudachi-based `/api/analyze`
- port only the token normalization behavior that still matters for the UI contract

### Phase 3

- add English token classifier and correction pipeline
- add backend dictionary lookup endpoint

### Phase 4

- remove silent browser fallback behavior
- remove or reduce unused browser-side dictionary and tokenizer dependencies

## Testing Strategy

### Backend Unit Tests

Cover:

- abbreviation recognition
- ordinary English word preservation
- built-in override behavior
- dictionary-corrected English behavior
- stable response schema for `/api/analyze`

### Integration Tests

Cover:

- `GET /api/health`
- `POST /api/analyze`
- `GET /api/dictionary`
- startup flow when `/api/health` is temporarily not ready
- frontend can render returned token structures
- frontend playback uses `tts_text` correctly

### Required Regression Samples

Include at least:

- `AI`
- `API`
- `CPU`
- `React`
- `Docker`
- `vintage`
- `browser`
- `Web`
- `Node.js`
- `iOS`
- `GitHub`
- `TypeScript`
- date expressions such as `4月` and `20日`
- particle-sensitive examples such as `は`

## Risks and Mitigations

### Risk: Sudachi output differs from current UI assumptions

Mitigation:

- normalize token output in the backend
- keep frontend rendering contract stable
- add regression tests on representative sentences

### Risk: English correction data is noisy

Mitigation:

- only use correction when confidence is high
- otherwise preserve original English

### Risk: Rust asset and dictionary loading complicates startup

Mitigation:

- keep the first version simple: serve static files from the repo and load dictionary files from disk
- avoid binary embedding in phase 1
- expose explicit health status

### Risk: Frontend still contains duplicated old logic

Mitigation:

- switch one entrypoint at a time
- remove or disable silent fallback paths early in the migration

## Success Criteria

- Ordinary English words are no longer read as letter-by-letter abbreviations by default.
- Known abbreviations still behave correctly.
- Known English technical terms and loanwords are read using common Japanese usage when confidence is high.
- Loanword and modern vocabulary segmentation is noticeably better than the current browser tokenizer.
- Clicking a token card returns dictionary data without loading large client-side dictionary blobs first.
- Backend startup failures are visible and actionable.
- The system runs as a local Rust service plus local dictionary files.

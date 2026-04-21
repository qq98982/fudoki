# Fudoki Sudachi Backend Design

Date: 2026-04-21

## Summary

Fudoki will move Japanese analysis from an in-browser tokenizer to a local backend service. The backend will serve the existing frontend static assets and expose analysis and dictionary APIs. The primary analysis engine will be `SudachiPy` with `sudachidict_full`. This change is intended to improve tokenization quality, English and loanword handling, dictionary lookup performance, and runtime reliability.

## Context

The current application is a static frontend that performs tokenization and reading generation in the browser with `kuromoji` and `kuroshiro`. This causes several user-visible issues:

- Ordinary English words are misclassified as abbreviations and are pronounced letter-by-letter.
- Tokenization quality is limited for loanwords, product names, and modern vocabulary.
- Dictionary lookup requires loading large client-side data and scanning it linearly.
- Initialization failure silently degrades to character-level fallback tokenization.
- Analysis is mostly button-driven or blur-driven instead of feeling local and immediate.

The user is comfortable running a local service before opening the frontend, so introducing a backend is acceptable.

## Goals

- Improve segmentation quality by replacing browser tokenization with `SudachiPy + sudachidict_full`.
- Fix English token handling so ordinary English words are not treated as abbreviations.
- Preserve correct behavior for real abbreviations such as `AI`, `API`, and `CPU`.
- Keep the current frontend UI and user workflow largely intact.
- Make dictionary lookup local and fast enough for interactive token-card usage.
- Remove silent quality degradation when the analysis engine fails to initialize.

## Non-Goals

- Rebuilding the frontend framework or redesigning the UI.
- Shipping a cloud-hosted service.
- Supporting arbitrary browser-only offline execution in this phase.
- Solving perfect English-to-katakana conversion for unknown words.

## Chosen Approach

The system will become a local web application with one backend process:

- `FastAPI` serves `index.html` and `/static/*`.
- `FastAPI` also exposes `/api/*` endpoints for analysis and dictionary lookup.
- `SudachiPy` with `sudachidict_full` becomes the primary tokenizer.
- Frontend analysis logic is switched from local `JapaneseSegmenter` calls to HTTP API calls.
- Existing browser TTS, document management, settings, and reading mode remain in the frontend.

This approach preserves the UI while moving the quality-critical language logic to a better local runtime.

## Alternatives Considered

### Option A: Keep pure frontend and improve dictionaries

This would reduce some lookup issues but would not fix the main source of bad English and loanword analysis. Tokenization quality would still be limited by the browser-side stack.

### Option B: Local backend with Sudachi

This is the chosen option. It improves the core analysis engine, simplifies future extensions, and keeps the user-visible UI stable.

### Option C: Full rewrite into a different app stack

This would create avoidable migration risk and provide little immediate value compared with replacing the analysis layer.

## Architecture

### Backend

Create a new `backend/` package with:

- `backend/app.py`
  - FastAPI app creation
  - static file mounting
  - API route registration
- `backend/analyzer.py`
  - Sudachi tokenizer wrapper
  - line-based analysis entrypoint
  - reading and POS normalization
- `backend/english.py`
  - English token classifier and correction pipeline
- `backend/dictionary.py`
  - local dictionary loading
  - query and lookup helpers
- `backend/models.py`
  - request and response schemas

### Frontend

Keep the current frontend files, but change the data source:

- `static/main-js.js`
  - `analyzeText()` will call `POST /api/analyze`
  - token-card translation loading will call `GET /api/dictionary`
- Existing rendering functions continue to consume token arrays with minimal structural change.

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

This is the main behavior change and will use an explicit precedence order.

### Classification Order

1. Abbreviation
2. Built-in override
3. Dictionary-corrected English term
4. Unknown English term

### Abbreviation Rule

Treat as abbreviation only when the token is a high-confidence abbreviation, such as:

- all uppercase alphabetic token like `AI`, `CPU`, `API`
- common mixed tokens with well-known uppercase semantics if explicitly whitelisted

Abbreviations keep letter-name pronunciation behavior.

### Built-in Override Rule

Use a curated built-in map for common technical words and product names where the application wants stable, predictable readings, such as:

- `React`
- `Docker`
- `TypeScript`
- `JavaScript`

### Dictionary-Corrected English Rule

If a token is alphabetic but not an abbreviation, query local correction data. If the backend can confidently map the English token to a Japanese equivalent or stable reading, use that reading and mark its source as dictionary-corrected.

### Unknown English Rule

If no reliable correction exists, preserve the original English token and do not fabricate katakana. This avoids the current failure mode where ordinary English words are pronounced letter-by-letter.

## Dictionary Sources

### Primary Analysis Dictionary

- `sudachidict_full`

This is the primary source for token boundaries and base linguistic analysis.

### Primary Lookup Dictionary

- JMdict-derived local data

This remains the main source for token-card meaning lookup.

### English Correction Data

The first implementation should support an offline local correction source that can be built into project data. Acceptable inputs include:

- user-provided local dictionary data
- official downloadable dictionary sources such as EDRDG JMdict
- supplementary Japanese dictionary data suitable for English-to-Japanese correction

The runtime system should not depend on parsing `MDX` directly in the browser. If an `MDX` file is used, it must be converted offline into a project-local queryable format first.

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
- `pos`
- `source`
- `confidence`

### `GET /api/dictionary?term=...`

Response fields should support the existing token-card UI:

- normalized query term
- `kanji`
- `kana`
- `senses`
- optional metadata about lookup source

## Error Handling

- If Sudachi initialization fails, backend startup should fail visibly instead of silently degrading into character tokenization.
- If dictionary data is unavailable, `/api/dictionary` should return a structured error or empty result, not hang.
- Frontend should show a user-visible failure state when `/api/analyze` cannot be reached.
- Frontend should not attempt browser-side tokenization fallback in this phase.

## Frontend Integration Changes

### Immediate Changes

- Replace `initSegmenter()` and browser `segment()` usage with a backend API client.
- Move translation lookup from `window.dictionaryService` to backend requests.
- Keep existing token rendering and TTS UI behavior.

### Transitional Compatibility

The old browser tokenization libraries may remain temporarily in the repo during migration, but they are no longer the primary analysis path. They should not be used as a silent fallback.

## Performance Expectations

- Analysis should feel local and immediate on typical note-sized input.
- Dictionary lookup should be fast enough for token-card interaction without loading the full dataset into browser memory.
- Because the app runs locally, larger local dictionaries are acceptable if they improve analysis quality.

## Migration Plan

### Phase 1

- Add local backend serving static assets and health endpoint.
- Wire frontend to backend connectivity checks.

### Phase 2

- Add Sudachi-based `/api/analyze`.
- Port current token normalization behavior that still matters, such as dates and display shaping, only where still necessary.

### Phase 3

- Add English token classifier and correction pipeline.
- Add backend dictionary lookup endpoint.

### Phase 4

- Remove silent browser fallback behavior.
- Remove or reduce unused browser-side dictionary and tokenizer dependencies.

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

- `POST /api/analyze`
- `GET /api/dictionary`
- frontend can render returned token structures

### Required Regression Samples

Include at least:

- `AI`
- `API`
- `CPU`
- `React`
- `Docker`
- `vintage`
- `browser`
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

### Risk: Frontend still contains duplicated old logic

Mitigation:

- switch one entrypoint at a time
- remove or disable silent fallback paths early in the migration

## Success Criteria

- Ordinary English words are no longer read as letter-by-letter abbreviations by default.
- Known abbreviations still behave correctly.
- Loanword and modern vocabulary segmentation is noticeably better than the current browser tokenizer.
- Clicking a token card returns dictionary data without loading large client-side dictionary blobs first.
- Backend startup failures are visible and actionable.

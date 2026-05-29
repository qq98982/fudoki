# Fudoki React Frontend Rewrite Design

## Summary

This design rewrites the current browser-side static JavaScript frontend into a React application while keeping Rust as the local backend. The rewritten app remains a local web app that runs quickly on Windows and Linux: Rust starts the local server, serves the React bundle, exposes APIs, and owns the local persistent store.

The first phase is intentionally narrower than the current product surface:

- Keep Rust as the analyzer, dictionary, and remote TTS backend.
- Replace the legacy monolithic frontend with a React + TypeScript app.
- Use local SQLite, owned by Rust, as the source of truth for documents, settings, analysis cache, and TTS cache metadata.
- Keep multiple documents, but simplify the document-management UI to a compact list instead of the current folder-heavy structure.
- Keep Japanese analysis, dictionary lookup, system TTS, remote TTS, autosave, themes, language switching, and display toggles.
- Defer Firebase login, cloud sync, folder system, and the current PWA-heavy offline flow.
- Do not require Markdown for phase 1. The editor must be clean, stable, and upgradeable later.

The goal is not just a visual rewrite. The goal is to make logic, persistence, caching, and future extension points controllable.

## Best-Practice Refinements

After the initial design pass, the preferred implementation choices are:

- Rust is the only durable data owner.
- SQLite is accessed through repositories and transactions, not ad hoc handler code.
- frontend server state lives in TanStack Query, not a catch-all global store.
- Zustand stays small and is limited to shell, editor interaction, and playback state.
- the phase-1 service worker is removed entirely unless a concrete static-asset need proves it necessary.
- documents use stable globally unique IDs such as UUIDv7 or ULID.
- document saves use revision-based optimistic concurrency so multiple tabs cannot silently overwrite each other.
- sample articles are treated as templates/import sources, not primary user documents mixed into the main list.

## Goals

- Build a maintainable React frontend with clear state boundaries.
- Move durable user data out of browser `localStorage` and into Rust-managed SQLite.
- Preserve core user value:
  - text/article input
  - multiple documents
  - autosave
  - Japanese analysis
  - dictionary details
  - system TTS
  - remote TTS
  - language/theme/display settings
- Make caching explicit and inspectable instead of implicit and scattered.
- Support a later second phase for:
  - Markdown editor support
  - cloud sync
  - desktop shell packaging

## Non-Goals For Phase 1

- Firebase login
- cloud sync
- folder hierarchy
- favorites-heavy organization model
- service-worker-first offline mode
- pure Rust desktop UI
- full Markdown/EasyMDE parity

## Approaches Considered

### 1. React frontend + Rust backend local web app

This keeps the current deployment model but replaces the frontend architecture.

Pros:

- Lowest migration risk from the current codebase
- Best fit for UI complexity and future extensibility
- Fast local runtime on Windows and Linux
- Keeps Rust focused on analysis, TTS, persistence, and serving
- Can later be wrapped by Tauri without rewriting the React app

Cons:

- Still runs in a browser rather than a native desktop window
- Browser APIs still matter for system TTS and some UI behavior

### 2. Tauri + React + Rust immediately

Pros:

- More desktop-like packaging from day one
- Easier future OS integrations

Cons:

- Adds packaging and runtime integration complexity too early
- Slows down the core rewrite

### 3. Pure Rust desktop UI

Pros:

- Single-language stack
- More native-app identity

Cons:

- High UI rewrite cost
- Worse fit for rich editor, complex presentation logic, and iterative frontend work
- Slower path to a stable replacement

### Decision

Choose approach 1 for phase 1.

The product should become a local web app with:

- React + TypeScript frontend
- Rust API/backend
- SQLite local database
- optional Tauri wrapper later, not now

## Product Shape

Phase 1 becomes a focused local reading and analysis workstation:

- a simplified document list on the left
- a large article/editor workspace in the center
- an inspector panel on the right for analysis, dictionary, TTS, and settings

The information architecture stays familiar to current users, but the logic is separated into bounded modules instead of one giant browser script.

## High-Level Architecture

### Runtime

- Rust process starts the local app.
- Rust serves the built React assets.
- Rust exposes JSON/stream APIs under `/api/*`.
- React runs as a single-page app in the browser.
- SQLite is the durable store.
- browser storage is reduced to temporary migration/bootstrap helpers only

### Dependency Direction

The dependency direction should stay inward:

- React components depend on feature hooks and view models.
- Feature hooks depend on API clients, stores, and editor/TTS adapters.
- Rust HTTP handlers depend on use-case services.
- Use-case services depend on repository traits.
- SQLite, filesystem, and upstream TTS adapters sit at the outer edge.

This keeps UI, business rules, and infrastructure replaceable.

## Frontend Architecture

### Tech Stack

- React
- TypeScript
- Vite
- no router by default; add routing only if the app grows real route boundaries
- TanStack Query for server-backed state and caching
- Zustand only for local UI/application state that should not be modeled as fetch cache
- CodeMirror 6 as the initial editor engine in plain-text mode

CodeMirror 6 is chosen because it is a better long-term boundary than a raw `textarea` or EasyMDE:

- good React integration
- good control over selection, decorations, and events
- clean upgrade path to Markdown later
- avoids carrying phase-1 UI debt from EasyMDE

### Frontend Module Boundaries

#### `library`

Owns:

- simplified document list
- active document selection
- search, sort, create, rename, delete

Does not own:

- analysis payloads
- TTS playback
- raw browser persistence

#### `editor`

Owns:

- editor instance lifecycle
- editor content draft state
- dirty state
- autosave triggers
- save queue coordination
- read/edit presentation modes

Does not own:

- document persistence implementation
- analysis requests

#### `analysis`

Owns:

- manual analyze action
- debounced analyze trigger policy
- cached result retrieval
- current token selection
- line/token display shaping

Does not own:

- raw document save lifecycle
- TTS playback state

#### `dictionary`

Owns:

- token click lookup
- request state
- detail presentation

#### `tts`

Owns:

- provider selection
- system TTS state
- remote TTS request lifecycle
- remote playback state
- progress and error state

Does not own:

- document persistence
- settings persistence

#### `settings`

Owns:

- language
- theme
- token display toggles
- auto-read and repeat preferences
- TTS defaults

#### `shell`

Owns:

- layout state
- panel visibility
- mobile drawer/sheet behavior
- bootstrapping and legacy migration handoff

### State Ownership Rules

These should be treated as hard constraints:

- documents, settings, cached analysis payloads, and provider metadata belong in TanStack Query
- editor draft text, active token selection, panel visibility, and playback progress may live outside Query
- no feature writes directly to `localStorage`
- no React component calls `fetch` directly; all requests go through typed API clients
- no frontend store becomes a second database; if state must survive reload, it belongs in Rust/SQLite

## Backend Architecture

### Rust Responsibilities

Rust should own:

- document persistence
- settings persistence
- SQLite migrations
- analysis service
- dictionary service
- remote TTS provider integration
- remote TTS durable cache index
- static asset serving
- one-time legacy browser-data import
- SQLite connection lifecycle and tuning

Rust should not own:

- system TTS playback
- editor UI logic
- token presentation rules that are purely visual

### Proposed Rust Modules

The backend should be split so the structure reflects the domain, not the framework:

```text
src/
  app.rs
  api/
    analysis.rs
    dictionary.rs
    documents.rs
    settings.rs
    tts.rs
    migration.rs
  domain/
    documents.rs
    settings.rs
    analysis.rs
    tts.rs
  storage/
    db.rs
    migrations.rs
    documents_repo.rs
    settings_repo.rs
    analysis_cache_repo.rs
    tts_cache_repo.rs
  services/
    analyzer.rs
    dictionary.rs
    legacy_import.rs
    tts_remote.rs
```

The current `analyzer`, `dictionary`, and `tts` code can be moved into these boundaries incrementally instead of rewritten from scratch.

### SQLite Access Pattern

Best-practice choice for this project:

- use `rusqlite`
- run SQLite in WAL mode
- enable foreign keys
- set a sensible busy timeout
- execute writes inside explicit transactions
- keep SQL and repository code in the storage layer

This is preferable to scattering raw SQL through handlers, and it is a better fit for an embedded local database than pretending SQLite is a remote async data service.

## Persistence Design

### SQLite As Source Of Truth

SQLite becomes the durable source of truth for local user data. This is a major improvement over the current frontend, where document state and settings are spread across many `localStorage` keys.

Recommended data location:

- Linux: under the standard app data directory, such as `~/.local/share/fudoki/`
- Windows: under the standard per-user app data directory, such as `%APPDATA%\\Fudoki\\`

Recommended files:

- `fudoki.db`
- `tts-cache/` directory for remote audio files if durable audio caching is enabled

### Tables

#### `documents`

Purpose: user-authored or imported local documents

Suggested columns:

- `id TEXT PRIMARY KEY`
- `title TEXT NOT NULL`
- `title_mode TEXT NOT NULL` with values such as `auto` or `custom`
- `content TEXT NOT NULL`
- `source_kind TEXT NOT NULL` with values such as `user` or `sample`
- `created_at INTEGER NOT NULL`
- `updated_at INTEGER NOT NULL`
- `revision INTEGER NOT NULL`
- `deleted_at INTEGER NULL`

Notes:

- `revision` increments on every persisted content change.
- document IDs should be UUIDv7 or ULID.
- `title` is auto-derived from the first meaningful line until the user customizes it.
- soft delete is optional, but leaving room for it helps later undo/sync work.

#### `settings`

Purpose: durable user preferences

Suggested columns:

- `key TEXT PRIMARY KEY`
- `value_json TEXT NOT NULL`
- `updated_at INTEGER NOT NULL`

This keeps schema churn low for phase 1.

#### `analysis_cache`

Purpose: cached analyzer results

Suggested columns:

- `cache_key TEXT PRIMARY KEY`
- `document_id TEXT NULL`
- `document_revision INTEGER NULL`
- `content_hash TEXT NOT NULL`
- `analyzer_version TEXT NOT NULL`
- `result_json TEXT NOT NULL`
- `created_at INTEGER NOT NULL`
- `last_used_at INTEGER NOT NULL`

Key policy:

- cache by content hash and analyzer version
- optionally associate the cache row with `document_id` and `document_revision`

#### `tts_audio_cache`

Purpose: durable remote TTS cache index

Suggested columns:

- `cache_key TEXT PRIMARY KEY`
- `provider TEXT NOT NULL`
- `document_id TEXT NULL`
- `document_revision INTEGER NULL`
- `cache_scope_version TEXT NULL`
- `text_hash TEXT NOT NULL`
- `model TEXT NOT NULL`
- `voice TEXT NOT NULL`
- `format TEXT NOT NULL`
- `speed_hundredths INTEGER NOT NULL`
- `content_type TEXT NOT NULL`
- `audio_path TEXT NOT NULL`
- `expires_at INTEGER NOT NULL`
- `created_at INTEGER NOT NULL`
- `last_used_at INTEGER NOT NULL`

The actual audio bytes should live on disk, not as large BLOB rows in SQLite.

#### `app_meta`

Purpose: migration and schema metadata

Suggested columns:

- `key TEXT PRIMARY KEY`
- `value_json TEXT NOT NULL`

Use this for:

- schema version
- analyzer version
- legacy import completion marker

## API Design

### Existing APIs To Keep

- `GET /api/health`
- `GET /api/dictionary`
- `GET /api/tts/providers`
- `POST /api/tts/speak`

### New Phase-1 APIs

#### Documents

- `GET /api/documents`
- `POST /api/documents`
- `GET /api/documents/:id`
- `PUT /api/documents/:id`
- `DELETE /api/documents/:id`

Optional but useful:

- `POST /api/documents/:id/duplicate`

Document writes should use optimistic concurrency. The update request should include the caller's last known revision, and the backend should reject stale writes instead of silently overwriting:

```json
{
  "title": "optional visible title",
  "title_mode": "auto",
  "content": "...",
  "expected_revision": 7
}
```

On success, the backend returns the new canonical document payload with the incremented revision.

#### Settings

- `GET /api/settings`
- `PUT /api/settings`

#### Analysis

Keep `POST /api/analyze`, but extend the request shape so the backend can use durable cache more effectively:

```json
{
  "document_id": "doc-123",
  "document_revision": 7,
  "text": "..."
}
```

Optional cached-result endpoint:

- `GET /api/documents/:id/analysis?revision=7`

This lets the frontend restore a saved document and reuse the last good analysis without re-running the analyzer on every document switch.

#### Legacy Migration

- `POST /api/migrations/legacy-browser-data`

The request contains a normalized snapshot of legacy browser data from the current frontend.

## Caching Strategy

Phase 1 should use three cache layers, each with a clear owner.

### 1. React in-memory cache

Owned by TanStack Query.

Use for:

- documents list
- document detail fetches
- settings
- dictionary lookups
- TTS provider metadata
- cached analysis reads

This cache is disposable and should never be treated as durable data.

### 2. Rust durable cache in SQLite/filesystem

Owned by Rust.

Use for:

- documents
- settings
- analysis cache
- remote TTS cache metadata
- optional remote TTS audio files

This is the real local source of truth.

### 3. Browser ephemeral storage

Minimize this aggressively.

Use only for:

- one-time legacy import bookkeeping
- ultra-short-lived shell state if needed

Do not keep primary documents, settings, or cache indexes here after migration.

### Service Worker Policy

The current service worker should be removed in phase 1.

Reasons:

- this is a local app, not a network-first public PWA
- stale frontend assets are a bigger risk than the value the current service worker provides
- SQLite already becomes the durable data owner, so browser offline caching should not pretend to be a persistence layer

If static-shell caching becomes necessary later, it can return in a much smaller, static-assets-only form.

## Legacy Data Migration

### Requirement

Current users must be able to open the new frontend in the same browser and keep their existing local documents and settings.

### Migration Flow

1. React boots and calls a lightweight bootstrap endpoint.
2. Rust reports whether local SQLite already contains migrated data.
3. If not migrated, React reads the current legacy browser keys:
   - document data from `texts` and `activeId`
   - settings from current keys such as theme, language, display toggles, TTS selections, sorting preference, and related retained settings
4. React sends a normalized snapshot to `POST /api/migrations/legacy-browser-data`.
5. Rust validates and imports the snapshot into SQLite in one transaction.
6. Rust records legacy import completion in `app_meta`.
7. React invalidates boot queries and reloads from SQLite-backed APIs.

### Migration Rules

- import current legacy keys, not every historical alias
- convert array-backed content into a single canonical text string
- preserve timestamps when available
- preserve the current active document if it still exists after import
- store retained settings only; do not import login/sync-related keys
- once migration succeeds, the new app should stop reading legacy browser data entirely

### Migration Safety

- import should be idempotent
- import should reject malformed payloads instead of partially writing
- backend should keep a small import summary for diagnostics

## UI Design

### Layout

#### Left rail

Compact document list with:

- new document
- search
- recent documents
- rename
- delete
- sort by updated time

No folders, no sync entry points, no login UI.

Sample content, if retained, should be exposed as templates or an import action, not mixed into the primary user document rail.

#### Center workspace

Large editorial writing/reading surface.

Phase-1 editor behavior:

- plain-text article input
- auto-save
- explicit analyze action
- optional debounced re-analysis
- clear reading presentation

Autosave should use a single-flight save coordinator:

- debounce while typing
- flush immediately on document switch
- flush on tab close or visibility loss when possible
- never allow overlapping saves for the same document

#### Right inspector

Structured side panel with stable sections:

- analysis
- dictionary
- TTS
- settings

This removes the current problem where these controls are spread across many ad hoc places.

### Visual Direction

The rewritten frontend should look intentional rather than generic.

Recommended direction:

- editorial workspace instead of a dashboard aesthetic
- warm paper-toned default theme
- strong contrast between content surface and inspector rail
- expressive typography with Japanese-friendly fonts, such as a serif for reading surface and a sans-serif for controls
- generous whitespace and calmer control density than the current UI

### Responsiveness

On narrow screens:

- document rail becomes a drawer
- inspector becomes a bottom sheet or tabbed panel
- editor remains the primary visible surface

## TTS Design

### System TTS

System TTS remains browser-driven and frontend-owned.

The React app should wrap Web Speech API calls behind a dedicated adapter so components do not manipulate `speechSynthesis` directly.

### Remote TTS

Remote TTS remains Rust-owned:

- provider metadata from Rust
- upstream calls from Rust
- durable audio-cache indexing in Rust

The React app only requests speech, manages playback state, and displays errors or progress.

### Playback Policy

- token playback defaults to system TTS
- line/full playback may use system or remote TTS depending on selected provider
- changing provider or voice updates persisted settings through the backend

## Analysis Design

### Trigger Policy

Support both:

- explicit analyze button
- optional debounced analyze-on-idle

The default should favor predictable behavior over noisy background work.

Recommended default:

- save immediately with debounce
- analyze when the user clicks analyze
- optionally auto-analyze after a short idle threshold once the main rewrite is stable

This is preferable to the current pattern where multiple concerns are mixed together in browser code and analysis re-trigger logic is difficult to reason about.

### Result Ownership

The backend returns structured token data as it does today, but the frontend owns only view shaping, not durable storage.

When a document is reopened:

- try to load cached analysis for the saved revision
- only re-run analysis if cache is absent or stale

## Future Sync Boundary

Cloud sync is deferred, but phase 1 should avoid painting itself into a corner.

The correct boundary is:

- React talks only to backend document/settings APIs
- backend repositories hide whether storage is local only or local + remote later

That means phase 1 does not need sync code, but it does need interfaces that separate:

- document storage
- settings storage
- remote sync orchestration

from each other.

If sync returns later, the preferred next step is an explicit outbox or change-log design rather than letting the frontend talk directly to a cloud SDK again.

## Error Handling

### Frontend

- display request-state UI per panel instead of one global error bucket
- keep editor usable even when analysis or remote TTS fails
- never lose typed content because another subsystem failed
- surface save conflicts clearly if another tab changed the same document

### Backend

- fail document writes atomically
- return structured error bodies
- distinguish user-facing validation errors from infrastructure failures
- log migration, SQLite, and remote TTS failures with enough metadata to debug

## Testing Strategy

### Rust

- repository tests against temporary SQLite databases
- migration tests for schema creation and upgrades
- legacy-import tests using real legacy payload shapes
- API integration tests for document, settings, analyze, dictionary, and TTS endpoints
- remote TTS cache tests for revision invalidation and scope invalidation

### Frontend

- component tests for document rail, editor shell, inspector panels
- store tests for editor, analysis, TTS, and settings state
- integration tests for bootstrap + migration flow
- interaction tests for autosave, document switch, and analyzer restore

### End-to-End

Add a small local smoke suite that verifies:

- first boot with no data
- boot with legacy browser data
- create/edit/switch/delete documents
- analyze text
- dictionary lookup
- system TTS controls render correctly
- remote TTS provider discovery works when configured

## Rollout Plan

### Phase 1 outcome

At the end of the rewrite, the app should be:

- React on the frontend
- Rust on the backend
- SQLite for local persistence
- simplified but stable in content management
- visually cleaner than the current app
- free of login/sync dependencies

### Deferred follow-ups

- Markdown mode on top of the editor adapter
- cloud sync
- Tauri packaging
- richer document organization

## Decision Summary

- Keep a local web app architecture.
- Use React + TypeScript for the new frontend.
- Keep Rust as the backend and static asset server.
- Use SQLite as the local source of truth.
- Use `rusqlite` with WAL mode and repository boundaries for local persistence.
- Simplify document management to a compact multi-document list.
- Remove login and sync from phase 1.
- Start with a clean text/article editor, not full Markdown parity.
- Keep analysis, dictionary, system TTS, and remote TTS.
- Remove the phase-1 service worker and reduce browser-side persistence to migration-only bookkeeping.
- Use revision-based saves and stable document IDs so future sync and multi-tab safety stay viable.

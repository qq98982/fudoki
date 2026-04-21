# Fudoki OpenAI-Compatible TTS Design

Date: 2026-04-21

## Summary

Fudoki will add an optional online text-to-speech path that works with OpenAI-compatible `audio/speech` providers while preserving the current browser-based system TTS path. The feature will be controlled by root-level `.env` configuration. When a complete compatible configuration is present, the UI will expose an `OpenAI-compatible` provider option. When it is absent, the UI will continue to show only the existing system voice flow.

The first implementation will support online playback for line and full-text playback. Token-level playback will remain on the existing system TTS path. Configuration errors must be visible both in settings and at playback time. The frontend must never receive the API key directly.

## Context

The current application reads text aloud through the browser's Web Speech API and the host operating system voice engine. That keeps setup simple, but it also means:

- the voice provider is not fixed or portable across browsers and operating systems
- speech quality varies significantly by environment
- there is no way to use a user-supplied OpenAI key or another compatible provider
- the current app has no `.env`-driven TTS configuration model

The user wants an online TTS option that can use:

- a custom OpenAI key
- another provider exposing an OpenAI-compatible base URL
- `.env`-based configuration

The user also wants this behavior:

- keep system TTS available
- only show the online provider if `.env` contains the required values
- if the online option exists but the values are wrong, show clear errors instead of hiding or silently falling back
- if `.env` defines defaults, use those defaults initially

## Goals

- Add an optional `OpenAI-compatible` online TTS provider.
- Keep the existing `System` provider and make it continue working without `.env`.
- Read online TTS configuration from the project root `.env`.
- Add a committed `.env.example` with documented example values during implementation.
- Expose the online provider in the UI only when required configuration values exist.
- Use `.env` defaults for provider selection and online TTS defaults when the user has not overridden them locally.
- Support online playback for line playback and full-text playback.
- Surface configuration and request failures clearly in settings and during playback.
- Keep API keys on the backend only.

## Non-Goals

- Replacing the existing system Web Speech path.
- Supporting token-level online playback in the first version.
- Achieving character-perfect resume semantics for remote audio playback.
- Building a provider marketplace or multi-provider account manager.
- Adding long-lived backend audio caching in the first version.

## Chosen Approach

Fudoki will use a dual-provider model:

- `system`
  - existing browser `speechSynthesis` flow
- `openai-compatible`
  - frontend requests a local Rust backend endpoint
  - backend reads `.env`
  - backend calls a compatible `POST /audio/speech` upstream
  - backend returns audio to the frontend for playback

This approach keeps the current local app behavior intact while adding a controlled online TTS path.

## Alternatives Considered

### Option A: Minimal-Intrusion Dual Path

Add a second remote playback path while keeping the current browser TTS behavior intact.

This is the chosen option because it minimizes risk to the existing speech flow and allows the feature to stay optional.

### Option B: Unified Frontend TTS Abstraction First

Rewrite the whole frontend playback layer around a provider-agnostic abstraction before adding remote TTS.

This would be architecturally cleaner long-term, but it increases first-version scope and risks regressions in the already-working system voice flow.

### Option C: Backend-Centric Audio Orchestration

Move all speech behavior into the backend and make the frontend only consume audio.

This is not appropriate for the current application because the existing system voice path is inherently browser-hosted and does not need backend orchestration.

## Configuration Model

The backend will load `.env` at startup. The first version will use these variables:

```dotenv
FUDOKI_TTS_OPENAI_BASE_URL=https://api.openai.com/v1
FUDOKI_TTS_OPENAI_API_KEY=your_api_key_here
FUDOKI_TTS_OPENAI_MODEL=gpt-4o-mini-tts
FUDOKI_TTS_OPENAI_VOICE=alloy
FUDOKI_TTS_OPENAI_FORMAT=mp3
FUDOKI_TTS_DEFAULT_PROVIDER=openai-compatible
```

### Required Values

The online provider is considered configured only when all of the following are present:

- `FUDOKI_TTS_OPENAI_BASE_URL`
- `FUDOKI_TTS_OPENAI_API_KEY`
- `FUDOKI_TTS_OPENAI_MODEL`

If any required value is missing:

- the online provider is not shown in the frontend
- the app behaves like the current system-only implementation

### Optional Values

These values define backend defaults when present:

- `FUDOKI_TTS_OPENAI_VOICE`
- `FUDOKI_TTS_OPENAI_FORMAT`
- `FUDOKI_TTS_DEFAULT_PROVIDER`

### Default Precedence

The runtime precedence will be:

1. user-local saved value
2. backend default from `.env`
3. frontend or backend safe fallback

For provider selection specifically:

1. user-local saved provider
2. `.env` default provider if valid
3. `openai-compatible` if required online config exists
4. `system`

This satisfies the user request that `.env` values act as defaults while still allowing later user overrides.

## Backend Architecture

Add a focused TTS module to the Rust backend.

### New Module

- `src/tts.rs`
  - `.env`-backed config loading
  - provider visibility and status resolution
  - compatible upstream request building
  - response and error normalization

### Router Changes

- `src/app.rs`
  - add `GET /api/tts/providers`
  - add `POST /api/tts/speak`
  - add shared TTS state to app state

### Startup Changes

- `src/main.rs`
  - load `.env` during startup

### Dependency Changes

- `Cargo.toml`
  - add `dotenvy`
  - add `reqwest`
  - add any minimal supporting crate needed for content-type handling or streamed bytes

## API Design

### `GET /api/tts/providers`

Purpose:

- tell the frontend which provider options are visible
- expose backend defaults derived from `.env`
- expose provider health and error summaries suitable for display

Response shape:

```json
{
  "providers": [
    {
      "id": "system",
      "label": "System",
      "status": "available"
    },
    {
      "id": "openai-compatible",
      "label": "OpenAI-compatible",
      "status": "available",
      "defaults": {
        "voice": "alloy",
        "format": "mp3"
      }
    }
  ],
  "default_provider": "openai-compatible"
}
```

Status values in the first version should stay simple:

- `available`
- `hidden`
- `request_failed`

`hidden` is a backend-internal concept; providers returned to the frontend should only be the visible ones. The frontend should not receive a provider row for a missing configuration case.

If the online provider has complete required config but the most recent request failed, `GET /api/tts/providers` may include a short non-secret error summary so settings can display that state.

### `POST /api/tts/speak`

Purpose:

- synthesize remote speech for `openai-compatible`
- return audio bytes to the frontend

Request shape:

```json
{
  "provider": "openai-compatible",
  "text": "日本語の文章",
  "voice": "alloy",
  "format": "mp3",
  "speed": 1.0
}
```

Backend behavior:

1. validate that the provider is enabled
2. resolve model and default values from configuration
3. send a compatible upstream request to `{base_url}/audio/speech`
4. return audio bytes with the correct content type

On failure, the endpoint returns structured JSON with an explicit user-displayable message and an appropriate status code.

Example failure shape:

```json
{
  "error": {
    "code": "tts_request_failed",
    "message": "TTS request failed: 401 Unauthorized"
  }
}
```

Secrets must never be returned.

## Frontend Behavior

### Settings UI

Add a provider selector to the existing voice settings surfaces:

- main settings modal
- header controls where appropriate

Behavior:

- `System` is always shown
- `OpenAI-compatible` is shown only when backend configuration is complete
- if online provider exists and has a known error state, settings display that state clearly

The first version should prefer a simple UI:

- provider selector
- provider status text for online mode
- existing system voice selector remains relevant only for `System`
- online `voice` may be user-editable in a later step if implementation cost stays reasonable

If scope pressure appears during implementation, the first version may keep remote `model` fixed to `.env` and only expose remote `voice` later. The primary requirement is compatibility and reliable playback, not a large settings surface.

### Playback Scope

System provider:

- keep current behavior
- keep current token, line, and full-text playback

Online provider:

- support line playback
- support full-text playback
- token playback remains on system TTS in the first version

If the current provider is `openai-compatible` and the user triggers token playback, the implementation should use the existing system path explicitly rather than silently sending a remote request.

## Runtime Playback Model

The frontend should keep system and remote playback state separate.

### System State

Keep the existing `speechSynthesis`-driven state variables and behavior with minimal change.

### Remote State

Introduce a distinct remote playback state machine:

- `idle`
- `loading`
- `playing`
- `error`

Use `HTMLAudioElement` for remote playback.

Rules:

- only one provider can be actively playing at a time
- starting remote playback cancels system speech first
- starting system playback stops remote audio first
- switching remote settings while remote audio is playing stops current playback and restarts the target text from the beginning of that line or full-text request

The first version does not need exact per-character continuation for remote audio.

## Error Handling

This feature must be explicit about failure.

### Missing Configuration

If required `.env` values are incomplete:

- the online provider does not appear in the frontend

### Invalid Configuration or Upstream Failure

If required values exist but the upstream request fails:

- settings must display an error state for the online provider
- playback attempts must display a direct, understandable error
- the app must not silently fall back to system TTS

### Error Message Rules

- safe to show to the user
- no secrets
- concise
- include status signal where useful, for example `401 Unauthorized`

## File-Level Change Plan

The implementation should stay focused on these files:

- `src/main.rs`
- `src/app.rs`
- `src/tts.rs`
- `Cargo.toml`
- `static/js/tts.js`
- `static/main-js.js`
- `static/js/backend-api.js` if request helpers are extracted there
- `index.html`
- `.env.example`
- `README.md`

Avoid unrelated refactors while implementing this feature.

## Testing Strategy

The implementation plan should include both backend and frontend coverage.

### Backend Tests

- provider visibility when required `.env` values exist
- provider hidden when required `.env` values are incomplete
- request failure normalization from upstream responses
- `POST /api/tts/speak` returns audio response metadata when upstream succeeds

### Frontend Tests

- online provider option is absent when backend does not expose it
- online provider defaults are used when present and no local override exists
- provider error state is rendered in settings
- playback failure surfaces a user-visible error
- token playback in online mode still uses the system path

## Security and Privacy

- API keys stay in backend environment only
- frontend never stores secrets
- error messages must not leak authorization headers or raw credentials
- no remote TTS request should be made directly from the browser

## Rollout Notes

The implementation should preserve the current app as a working default. A user without `.env` configuration should experience no behavior change. A user with valid `.env` configuration should see the additional provider and, by default, use `.env` values unless they have already chosen different local settings.

## Open Questions Resolved

- Should system TTS remain available: yes
- Should the online provider appear only when config exists: yes
- Should invalid config stay visible and fail explicitly: yes
- Should `.env` values act as defaults: yes
- Should the first version support token-level remote playback: no
- Should the first version target generic OpenAI-compatible `base_url + key + model`: yes

# React Rust SQLite Rewrite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the legacy static frontend with a React + TypeScript app backed by Rust APIs and local SQLite persistence while preserving analysis, dictionary, and TTS.

**Architecture:** Rust becomes the only durable data owner and serves both APIs and built frontend assets. React owns presentation and interaction flow. SQLite stores documents, settings, cached analysis, and remote TTS cache metadata.

**Tech Stack:** Rust, Axum, rusqlite, React, TypeScript, Vite, TanStack Query, Zustand, CodeMirror 6

---

### Task 1: Add persistent backend foundation

**Files:**
- Create: `src/storage/mod.rs`
- Create: `src/storage/db.rs`
- Create: `src/storage/documents_repo.rs`
- Create: `src/storage/settings_repo.rs`
- Modify: `src/lib.rs`
- Modify: `Cargo.toml`
- Test: `tests/documents_api.rs`
- Test: `tests/settings_api.rs`

- [ ] **Step 1: Write failing backend API tests for documents and settings**

- [ ] **Step 2: Run those tests to verify they fail because routes/storage are missing**

- [ ] **Step 3: Implement SQLite bootstrap, migrations, repositories, and API routes**

- [ ] **Step 4: Re-run the new backend tests until they pass**

### Task 2: Add legacy browser-data import

**Files:**
- Create: `src/api/migration.rs`
- Modify: `src/app.rs`
- Modify: `src/models.rs`
- Test: `tests/legacy_migration_api.rs`

- [ ] **Step 1: Write failing migration tests for importing legacy documents and settings**

- [ ] **Step 2: Run migration tests to verify they fail correctly**

- [ ] **Step 3: Implement normalized import request parsing and transactional import**

- [ ] **Step 4: Re-run migration tests until they pass**

### Task 3: Add analysis/document integration surfaces

**Files:**
- Modify: `src/models.rs`
- Modify: `src/app.rs`
- Modify: `tests/analyze_api.rs`

- [ ] **Step 1: Write failing test for extended analyze request accepting document metadata**

- [ ] **Step 2: Run that test and verify failure**

- [ ] **Step 3: Implement request/model updates while preserving existing analyzer behavior**

- [ ] **Step 4: Re-run analyze tests until they pass**

### Task 4: Scaffold React frontend

**Files:**
- Create: `frontend/package.json`
- Create: `frontend/tsconfig.json`
- Create: `frontend/vite.config.ts`
- Create: `frontend/index.html`
- Create: `frontend/src/main.tsx`
- Create: `frontend/src/app/App.tsx`
- Create: `frontend/src/app/providers.tsx`
- Create: `frontend/src/styles/*.css`
- Create: `frontend/src/features/**`
- Test: `frontend/src/**/*.test.tsx`

- [ ] **Step 1: Write a failing frontend smoke test for the new app shell**

- [ ] **Step 2: Run the frontend test to verify failure**

- [ ] **Step 3: Implement the React shell, document rail, editor surface, and inspector skeleton**

- [ ] **Step 4: Re-run the smoke test until it passes**

### Task 5: Wire frontend features to the backend

**Files:**
- Create: `frontend/src/lib/api.ts`
- Create: `frontend/src/features/documents/**`
- Create: `frontend/src/features/editor/**`
- Create: `frontend/src/features/analysis/**`
- Create: `frontend/src/features/dictionary/**`
- Create: `frontend/src/features/tts/**`
- Create: `frontend/src/features/settings/**`
- Test: `frontend/src/lib/api.test.ts`

- [ ] **Step 1: Write failing tests for API client behaviors and at least one document interaction**

- [ ] **Step 2: Run frontend tests to verify failure**

- [ ] **Step 3: Implement queries, mutations, autosave coordination, and inspector interactions**

- [ ] **Step 4: Re-run frontend tests until they pass**

### Task 6: Serve the built frontend and update local tooling

**Files:**
- Modify: `src/app.rs`
- Modify: `tests/static_serving.rs`
- Modify: `package.json`
- Modify: `run.sh`
- Modify: `run.ps1`

- [ ] **Step 1: Write failing static-serving tests for the built frontend path**

- [ ] **Step 2: Run static-serving tests to verify failure**

- [ ] **Step 3: Implement built-asset serving and update scripts to build frontend before launch**

- [ ] **Step 4: Re-run static-serving tests until they pass**

### Task 7: Verify the integrated rewrite

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Run backend test suite**

- [ ] **Step 2: Run frontend test suite**

- [ ] **Step 3: Run frontend production build**

- [ ] **Step 4: Run a Rust build with the new asset-serving path**

- [ ] **Step 5: Update README with the new local workflow**

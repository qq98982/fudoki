# Dictionary Parts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the LFS-tracked Sudachi dictionary with ordinary Git-tracked parts and rebuild `resources/system.dic` locally before tests or app startup.

**Architecture:** Store `resources/system.dic` as an ignored generated artifact. Keep deterministic split files under `resources/system.dic.parts/`, plus a small manifest with expected size and sha256. A Node script assembles the dictionary only when missing or invalid.

**Tech Stack:** Node.js filesystem APIs, npm scripts, Git, Rust integration tests.

---

### Task 1: Resource Prepare Script

**Files:**
- Create: `scripts/prepare-system-dic.mjs`
- Create: `tests/frontend/system-dic-parts.test.mjs`
- Modify: `.gitignore`
- Modify: `package.json`

- [ ] Write a Node test that reads `resources/system.dic.parts/manifest.json`, verifies every listed part exists and is under 100 MiB, assembles bytes in memory, and checks sha256 and size.
- [ ] Add `scripts/prepare-system-dic.mjs` to assemble `resources/system.dic` from the manifest when the file is missing or has the wrong sha256.
- [ ] Add `resources/system.dic` to `.gitignore`.
- [ ] Add `prepare:system-dic` and run it before `frontend:build`, `start`, `dev`, `serve`, and `test`.

### Task 2: Replace LFS Dictionary

**Files:**
- Modify: `.gitattributes`
- Create: `resources/system.dic.parts/manifest.json`
- Create: `resources/system.dic.parts/system.dic.part.*`
- Remove from Git index: `resources/system.dic`

- [ ] Split `resources/system.dic` into 50 MiB parts under `resources/system.dic.parts/`.
- [ ] Remove `resources/system.dic` from the Git index while keeping the local file.
- [ ] Remove the LFS rule from `.gitattributes`.
- [ ] Verify `git lfs status` reports no objects to push.

### Task 3: History-Safe Push State

**Files:**
- Git history only

- [ ] Because `origin/master..master` contains an older commit that introduced the LFS pointer, rewrite the local unpushed history so `resources/system.dic` is never reachable as an LFS object.
- [ ] Preserve the merged React frontend commits and the `package.json` test-order fix.
- [ ] Verify with `git lfs status`, `git log origin/master..master -- resources/system.dic .gitattributes`, and `git push --dry-run origin master`.

### Task 4: Verification

**Files:**
- No additional files

- [ ] Run `npm test`.
- [ ] Run `git status --short --branch`.
- [ ] Run `git push --dry-run origin master`.

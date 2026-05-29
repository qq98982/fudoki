# In-App Analysis Cache Clear Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a safe page-level control that clears only stored analysis-cache entries so users can invalidate stale readings without touching documents, settings, or TTS data.

**Architecture:** Add a narrow backend endpoint that truncates the `analysis_cache` table through the existing repository boundary. Surface that action in the React `Settings` panel behind a confirmation dialog and show short success/error feedback in the panel.

**Tech Stack:** Rust, Axum, rusqlite, React, TypeScript, Vitest, Testing Library

---

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Cross-platform desktop tool for batch-converting graduation thesis Word documents (.doc/.docx/.wps) to PDF. Users select a root folder containing per-student subfolders; the app intelligently identifies the thesis file in each subfolder (ignoring unrelated documents like proposals, literature reviews, etc.) and converts them.

Full requirements and decided constraints are in `SPECIFICATION.md`.

## Tech Stack

- **Desktop framework:** Tauri 2 (Rust backend + web frontend)
- **Frontend:** Vue 3 or React (to be decided at project init)
- **Conversion engine:** Bundled LibreOffice Portable (embedded in installer, zero user setup). Platform-specific fast paths: MS Office COM on Windows, NSDocument on macOS — LibreOffice as cross-platform fallback.
- **Auto-update:** Tauri Updater plugin (`@tauri-apps/plugin-updater`), backed by GitHub Releases or CDN
- **Target platforms:** Windows 10/11, macOS 12+, Ubuntu 20.04+

## Key Domain Concepts

- **Root folder (根文件夹):** Top-level directory selected by the user
- **Student folder (学生文件夹):** Subdirectories under root, named by student name, student ID, or combinations
- **Thesis identification:** Rule engine with keyword matching (`毕业论文`, `thesis`, `毕业设计`, etc.), exclusion keywords (`开题报告`, `任务书`, `文献综述`, etc.), single-file inference, and version-preference logic (prefer `终稿`/`final`, then newest modified time). Strategy: **prefer recall over precision** — when uncertain, select all candidates and let the user deselect.
- **Supported formats:** `.doc`, `.docx`, `.wps` (WPS Office native format)

## Architecture Notes

The app has three main layers:
1. **Scanner** — walks the root folder, discovers student subfolders and their Word/WPS files (respects configurable search depth, default 2 levels)
2. **Identifier** — applies the rule engine to pick the thesis file from each subfolder; produces a result list with statuses: identified / multiple candidates / not found. When ambiguous, selects all candidates (recall > precision).
3. **Converter** — drives bundled LibreOffice headless (or platform-specific engine) to convert selected files; runs in background threads with progress reporting, cancellation support, and per-file error isolation. Concurrency is adaptive: `min(cores/2, 4)` by default, user-adjustable 1–8, auto-degrades to serial when available RAM < 1GB. Files >100MB are converted in a dedicated single-file queue with a 5-minute timeout.

## Important Constraints

- Conversion engine must be **embedded** — users should never need to install LibreOffice or any dependency themselves
- Never modify or delete original Word files
- All conversion is local — no files uploaded to remote servers
- Chinese filenames, spaces, and special characters in paths must be handled correctly
- Single file conversion failure must not abort the batch
- UI must remain responsive during conversion (async/background processing)
- Large files (>100MB): warn in preview, convert solo outside the concurrency pool, 5min timeout, pre-check disk space ≥ 3x file size

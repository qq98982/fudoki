# Fudoki Launcher Scripts Design

Date: 2026-04-21

## Summary

Fudoki will add platform-specific launcher scripts that automate local startup for the Rust backend build. The launchers will support Linux/macOS and Windows, check for required tools, ensure the Sudachi dictionary exists locally, automatically download the official Sudachi core dictionary when missing, build the Rust binary, and start the local server.

## Context

The Rust backend is now the primary runtime path for Fudoki. The current documented startup flow still expects users to know they must:

- install Rust and Node tooling
- provide `resources/system.dic`
- run `cargo run`

This is too manual for normal local use, especially on Windows. The user wants startup to be automated, including compilation and dictionary download, with different entrypoints for different platforms.

## Goals

- Provide a simple launcher entrypoint for Linux/macOS.
- Provide a simple launcher entrypoint for Windows PowerShell.
- Provide a simple launcher entrypoint for Windows `cmd.exe` / double-click usage.
- Automatically download the official Sudachi core dictionary if `resources/system.dic` is missing.
- Automatically run a Rust build before startup.
- Keep the launch command stable across normal local use.

## Non-Goals

- Background service management.
- Automatic updates of Rust, Node, or Cargo themselves.
- Downloading the latest dictionary dynamically on every run.
- Installing system dependencies.
- Converting the launcher into a desktop installer.

## Chosen Approach

Add three launcher files at repo root:

- `run.sh`
- `run.ps1`
- `run.bat`

Behavior:

1. Check required tools.
2. Ensure `resources/` exists.
3. If `resources/system.dic` is missing, download and install the official Sudachi core dictionary.
4. Run `cargo build`.
5. Start the application with `cargo run`.

`run.bat` remains a thin wrapper around `run.ps1`.

## Platform Strategy

### Linux/macOS

Use `run.sh` as the main entrypoint.

Requirements:

- `bash`
- `cargo`
- one of `curl` or `wget`
- one of `unzip`, `bsdtar`, or `tar` with zip support

### Windows PowerShell

Use `run.ps1` as the main Windows implementation.

Requirements:

- `powershell.exe` or PowerShell
- `cargo`
- built-in PowerShell web request and zip extraction support

### Windows Batch

Use `run.bat` as a convenience wrapper that forwards to PowerShell:

```bat
powershell -ExecutionPolicy Bypass -File "%~dp0run.ps1"
```

This keeps Windows logic in one real implementation while still supporting `cmd.exe` and double-click usage.

## Dictionary Download Policy

### Source

Use the official SudachiDict release assets from Works Applications.

### Variant

Use the `core` dictionary, not `small` or `full`.

Rationale:

- `core` offers a good quality/size tradeoff for local use
- it matches the launcher goal better than shipping a very large first-run download

### Version Selection

Download the latest official SudachiDict core release when the dictionary is missing.

Behavior:

- resolve the current latest release from the official upstream
- download the `core` dictionary asset from that release
- install it as `resources/system.dic`

Constraint:

- this lookup happens only when `resources/system.dic` is missing
- the launcher does not re-download on every startup

### Installed Path

Downloaded dictionary is installed as:

- `resources/system.dic`

The original archive name should not be used at runtime. The launcher normalizes the installed path so the Rust config remains stable.

## Build and Run Behavior

### Build

Always run:

```bash
cargo build
```

before startup.

This keeps behavior simple and avoids stale binary confusion.

### Run

Then run:

```bash
cargo run
```

The launcher should not add backgrounding logic or daemon behavior in this phase.

## Error Handling

### Missing Tools

If a required tool is missing:

- exit non-zero
- print a direct error with the missing tool name
- do not continue

### Dictionary Download Failure

If download, extract, or install fails:

- exit non-zero
- print the failing step
- do not start the server

### Existing Dictionary

If `resources/system.dic` already exists:

- skip download
- continue to build and run

### Build Failure

If `cargo build` fails:

- exit non-zero
- print the build failure
- do not attempt startup

## Files

### Create

- `run.sh`
- `run.ps1`
- `run.bat`

### Modify

- `README.md`

Potentially:

- add launcher verification tests if lightweight script checks can be automated

## README Changes

Document the new preferred startup commands:

- Linux/macOS: `./run.sh`
- Windows PowerShell: `.\run.ps1`
- Windows Command Prompt: `run.bat`

Also document:

- required tools
- first-run dictionary download behavior
- installed dictionary location

## Testing Strategy

### Shell Script Checks

Verify:

- existing dictionary skips download path
- missing dictionary triggers download logic
- missing tool exits clearly

### PowerShell Script Checks

Verify:

- existing dictionary skips download path
- missing dictionary triggers download logic
- wrapper invocation from `run.bat` is correct

### Manual Smoke Tests

At minimum:

- Linux/macOS: `./run.sh`
- Windows PowerShell: `.\run.ps1`
- Windows `cmd.exe`: `run.bat`

Expected:

- dictionary downloaded if missing
- Rust project builds
- server starts successfully

## Risks and Mitigations

### Risk: Upstream asset naming changes

Mitigation:

- query the latest release in a controlled way
- validate the selected asset before installation

### Risk: Platform-specific extraction commands differ

Mitigation:

- use simple, well-supported platform-native tools
- keep extraction logic per platform rather than forcing a single cross-platform script

### Risk: First run is slow

Mitigation:

- use `core` dictionary instead of `full`
- skip download once `resources/system.dic` exists

## Success Criteria

- A user can start Fudoki on Linux/macOS with `./run.sh`.
- A user can start Fudoki on Windows with either `.\run.ps1` or `run.bat`.
- Missing Sudachi dictionary is downloaded automatically from the latest official source.
- Rust build is run automatically before startup.
- Launch failure messages are specific and actionable.

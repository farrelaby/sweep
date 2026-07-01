# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.0] - 2026-07-01

### Added
- `dirsweep update` command for self-updating from GitHub releases
- `dirsweep uninstall` command for self-removing the binary
- Cross-platform self-delete/replace via `self-replace` crate
- Retry policy for update downloads (3 attempts on checksum mismatch)
- Release profile optimizations (41% smaller binary: 4.7 MiB → 2.8 MiB)
- Unit tests for commands module (19 tests)
- AGENTS.md for AI-assisted development

### Changed
- Skip `ProjectHeader` entries when navigating with arrow keys
- Walk recursively for orphan target directories

### Fixed
- Orphan target detection for nested directories
- Deduplication logic for overlapping target paths

## [0.1.2] - 2026-06-29

### Changed
- Renamed project from `sweep` to `dirsweep`

### Added
- Multi-channel deployment infrastructure (crates.io, npm)
- Platform-aware install scripts (Linux, macOS, Windows)
- CI checks for format, clippy, and tests

### Fixed
- Scoped secret env vars to jobs with `cicd` environment
- Dry run UI with artificial delay
- Live deletion progress in TUI

### Removed
- Dockerfile and Docker release workflow
- x86_64-apple-darwin target (CI runner hangs)

## [0.1.1] - 2026-06-28

### Fixed
- Include README.md in npm package files
- Regenerate lockfile before publish

### Changed
- Restructured README install section
- Added platform-aware install scripts

## [0.1.0] - 2026-06-27

### Added
- Initial TUI implementation with ratatui + crossterm
- Directory scanning with lock file detection
- Interactive tree view with selection
- Deletion support (trash or permanent)
- Size computation for target directories
- Monorepo detection (pnpm workspaces, Turborepo, Nx, Lerna)
- Support for 12 target directory types
- Support for 9 package managers

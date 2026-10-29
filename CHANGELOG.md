# Changelog

All notable changes to TokenRadar are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0] - 2026-10-30

The first public release: Claude Code's usage limits in the Windows tray.

### Added

- Tray icon with two rings: the weekly limit (outer) and the 5-hour window (inner), with the exact numbers and reset times in the tooltip.
- Tray panel: both windows, reset times, a projection of when a limit fills up, running sessions and the source of every number.
- Limit sources in order of trust: Claude Code's status line (`rate_limits`), the usage API (only with your consent) and an estimate from local logs, marked `≈`.
- Statusline bridge: `tokenradar statusline install` shows the change to `~/.claude/settings.json`, asks, keeps a backup and chains to an existing status line command.
- Incremental indexer for Claude Code's local logs that reads only usage and metadata fields.
- Notifications: thresholds, a projection warning, a last warning before full, reset announcements and quiet hours. Clicking a notification, also from the Action Center, opens the panel.
- Settings, a first-run onboarding window and a tray-pin helper.
- CLI: `status`, `limits`, `sessions` (with `--json`), `doctor` (privacy-safe diagnostics bundle) and `path add|remove`.
- Per-user installer that puts the CLI on the user `PATH`, and a portable zip.
- Signed automatic updates from GitHub Releases: a daily check that can be turned off, installed only when you click "Install and restart".
- English and Turkish UI.

[Unreleased]: https://github.com/umitkara/tokenradar/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/umitkara/tokenradar/releases/tag/v0.1.0

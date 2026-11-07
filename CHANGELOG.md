# Changelog

All notable changes to TokenRadar are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Statusline v2: an 8-cell bar on the limit windows, scoped weekly limits (e.g. Fable), context window, API-equivalent cost and the model. Choose the segments, their order, one or two lines and the bar in Settings or in `[statusline]` in `config.toml`, with a live preview.
- Every value on the statusline keeps its source and age: app values older than 10 minutes are dimmed, and the app's estimates are never shown there.
- Experimental Claude Code mod in `integrations/claude-code/`: a band above the prompt near a limit, a toast when one is crossed (only while the app is closed) and `/radar` for every limit with its source.
- Time mark on the statusline: `│` on the bar and `⏱43%` show how much of the window's time has passed. Turn it off with `pace = false` or in Settings.
- The Claude Code mod warns on the band when use runs 10 points ahead of the time line (not in the window's first day), and `/radar` adds a pace line and the weekly time line.
- `tokenradar status --json` includes `locale` (`en` or `tr`), the language the CLI resolved.
- `tokenradar history --json`: each limit's stored readings in its current window, thinned to at most 168 points, read-only from the database.
- The Claude Code mod goes visual: `/radar` opens a pane with limit cards and source badges, the weekly time strip, a usage chart against the time line (SVG on the desktop, VS Code and mobile; colored cells in the terminal), live sessions and the context window. The band is one bordered line with a mini bar and a Details button, and says once when a window resets. Without the statusline bridge, the mod shows a short status entry.
- The installer ships the Claude Code mod. Install or remove it from onboarding, Settings → Data sources or `tokenradar mod install|uninstall`: TokenRadar adds its folder to `env.CLAUDE_CODE_PLUGIN_DIRS` in Claude Code's `settings.json`, with a diff, confirmation and backup. Uninstalling TokenRadar takes it out again. Left at its default, the mod uses the CLI installed with the app.
- The tray panel has a second view, **Details**, next to the summary: each limit with its own source, projection and pace against the time gone, the weekly time strip, a usage chart against the time line, and the sessions with their API-equivalent cost and share bars. The panel opens on the view you chose last. The summary's bars gain the time mark.

### Changed

- The statusline drops the pie glyph; the bar takes its place. Without color (`NO_COLOR`) or in a narrow terminal the bar is left out.

### Fixed

- The tray panel scrolls when its content is taller than the window; before, many sessions were cut off at the bottom.

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

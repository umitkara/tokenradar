# Changelog

All notable changes to TokenRadar are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Statusline v2: an 8-cell bar on the limit windows, scoped weekly limits (e.g. Fable), context window, API-equivalent cost and the model. Choose the segments, their order, one or two lines and the bar in Settings or in `[statusline]` in `config.toml`, with a live preview.
- Every value on the statusline keeps its source and age: app values older than 10 minutes are dimmed, and the app's estimates are never shown there.
- Experimental Claude Code mod in `integrations/claude-code/`: a band above the prompt near a limit, a toast when one is crossed (only while the app is closed) and `/radar` for every limit with its source.
- Time mark on the statusline: `│` on the bar and `⏱ 43%` show how much of the window's time has passed. Turn it off with `pace = false` or in Settings.
- The Claude Code mod warns on the band when use runs 10 points ahead of the time line (not in the window's first day), and `/radar` adds a pace line and the weekly time line.
- `tokenradar status --json` includes `locale` (`en` or `tr`), the language the CLI resolved.
- `tokenradar history --json`: each limit's stored readings in its current window, thinned to at most 168 points, read-only from the database.
- The Claude Code mod goes visual: `/radar` opens a pane with limit cards and source badges, the weekly time strip, a usage chart against the time line (SVG on the desktop, VS Code and mobile; colored cells in the terminal), live sessions and the context window. The band is one bordered line with a mini bar and a Details button, and says once when a window resets. Without the statusline bridge, the mod shows a short status entry.
- The installer ships the Claude Code mod. Install or remove it from onboarding, Settings → Data sources or `tokenradar mod install|uninstall`: TokenRadar adds its folder to `env.CLAUDE_CODE_PLUGIN_DIRS` in Claude Code's `settings.json`, with a diff, confirmation and backup. Uninstalling TokenRadar takes it out again. Left at its default, the mod uses the CLI installed with the app.
- Statusline v3: the line is a list of items, each with its own format, label and color. New items for tokens, durations, lines changed, vim mode, agent, version, session id, the folder, the project folder and the git branch (read from `.git/HEAD`, no git process), plus line breaks, separators and custom text. Powerline and rounded Powerline themes, `color = "auto" | "always" | "never"`, five built-in presets and TOML export/import of the `[statusline]` table. Existing configs draw the same line as before.
- The tray panel has a second view, **Details**, next to the summary: each limit with its own source, projection and pace against the time gone, the weekly time strip, a usage chart against the time line, and the sessions with their API-equivalent cost and share bars. The panel opens on the view you chose last. The summary's bars gain the time mark.
- Statusline layout: separators are items you can remove one by one, a `space` item adds 1–8 blank columns, and a `flex` item fills the free width so that what follows sits on the right, or in the middle between two. A new **Split** preset puts the limits on the left and the model, folder and branch on the right. `margin` keeps extra columns free on the right. Existing configs get their separators as items when read and look as before.
- The statusline has its own Settings page with a live terminal preview, automatic saving and undo. Lines are cards: add and remove lines, drag items between them, and set a space's width, a separator's glyph (or every separator's at once) and the right margin there. Adding a value next to another value puts a separator between them.
- New statusline items: **daily budget** (what is left of the weekly limit per remaining day, `20%/day`), **burn rate** (the session's API-equivalent cost per hour, after five minutes), **cache ratio** (how much of the last request came from the prompt cache), the **session name** and the **output style**. The Detailed preset adds the daily budget.
- Limits and the context window can show what is **left** (`58% left`); the context window also the **tokens left** (`124k left`). A limit can show its reset as a local **clock time** (`Tue 04:20`) instead of a countdown. Bars come in 4, 8 or 12 cells and in three styles (blocks, thin, dots); the editor shows a sample of each.
- New statusline item **compactions**: how many times the session was compacted (`⟲ 3`, or `⟲ 2 auto 1 manual`), and the auto-compact window when you set one in Claude Code's settings or environment (`· 300k`, `· auto off`). The count needs the app running; without it, only the setting shows.
- Terminal font helper in the statusline settings: see whether a Nerd Font is installed, download and install CaskaydiaMono Nerd Font Mono (Nerd Fonts 3.5.1, checked against pinned SHA-256 hashes, for your Windows account only, no administrator rights) after you confirm, and select it in Windows Terminal or VS Code with a diff, confirmation and backup. Each terminal has its own undo, and **Restore previous state…** puts back every terminal font TokenRadar changed and removes the font it installed, after one summary. The statusline tells the app which terminal it runs in (Windows Terminal, VS Code or another), never more.
- Statusline icon sets: `icons = "unicode"` puts a few plain symbols in place of the labels (`⇡ 76k ⇣ 1.2k`, `⌂ alpha`), `icons = "nerd"` gives every value a Nerd Font icon. Your own label always wins, and `≈`, `API` and dimmed values look the same in every set. Pick the set in the statusline settings; the editor says when the font cannot draw it. Export and Import are now the two icon buttons at the end of the toolbar, and Presets sits next to Restore default.
- Two new bar styles: **capsule**, a rounded bar from Nerd Fonts 3.4+ glyphs that also draws inside Powerline blocks, and **solid**, colored background cells that need no special font. Both mark the time gone inside a cell, so the bar keeps its width.
- New statusline items from Claude Code's newer statusline fields: the session's **effort** (`high`, optionally with `fast` and `no thinking`), the **cache timer** (the local time the prompt cache goes cold, `cache →14:32 1h`), **cache misses** with the likely cause of the last one, the **recache cost** (`if cold 45k`), the **spend limit** behind a Claude apps gateway (`$ 42%` or `$21/$50`, with its reset), the open **pull request** with its review state (`#123 ✓`, GitLab `!45`), the **worktree** and the **repository** (`owner/name`). The model can show its effort, the folders the number of added folders (`alpha +2`) and the cache ratio the whole session's ratio. The Detailed preset adds effort and the cache timer.

### Changed

- **The database moves to schema v2.** On first start the app reads every transcript once more to pick up past compactions; this takes about as long as the first indexing. Version 0.1.0 cannot open a v2 database: to go back, delete the database and let 0.1.0 index again.
- The statusline leaves the last four columns free, which Claude Code cuts with `…`, and measures glyph widths per terminal cell.
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

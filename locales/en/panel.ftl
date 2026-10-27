# The tray panel (Step 10). Empty and error states follow design-language.md section 9.

panel-live-sessions = Running sessions
panel-top-sessions = Largest this window
panel-no-live-sessions = No Claude Code session is running.
panel-open-settings = Settings
panel-close = Close

## First start and missing data

# $pct is how much of the last 30 days has been processed.
panel-indexing = Processing the last 30 days of sessions… { PCT($pct) }
panel-indexing-ready = Current window ready
panel-no-claude = Claude Code was not found.
panel-no-claude-hint = Choose its folder, or set CLAUDE_CONFIG_DIR before starting TokenRadar.
panel-choose-folder = Choose folder
panel-no-limits = No limit data yet
panel-install-statusline = Install the statusline bridge (recommended)
panel-allow-api = Allow API access
panel-estimate-mode = Until then, limits are estimated from your session logs.

## Data problems. Calm wording, no error color.

panel-token-stale = Updates after you run Claude Code once.
# $ms is how long ago the last value arrived.
panel-offline = Offline · last value { DUR($ms) } ago
panel-schema-drift = Claude Code may have been updated; some data can't be read.
panel-diagnostics = Show diagnostics

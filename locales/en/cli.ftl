# The `tokenradar` console binary and the Claude Code statusline.

## Statusline. Keep these short: the line shares one terminal row with Claude Code.

sl-label-five-hour = 5h
sl-label-seven-day = 7d
sl-window = { $label } { PCT($pct) }
sl-resets-in = { DUR($ms, "compact") }
sl-eta = ▲{ DUR($ms, "compact") }
sl-session-share = ≈{ PCT($pct) } this session
sl-no-limits = no limit data yet

## Tables

col-limit = Limit
col-used = Used
col-resets = Resets in
col-source = Source
col-projection = At this rate
col-session = Session
col-project = Project
col-status = Status
col-share = Window share
col-cost = Cost

limit-session = 5-hour
limit-weekly-all = Weekly
limit-weekly-scoped = Weekly ({ $scope })
limit-other = { $kind }

source-statusline = statusline
source-api = API
source-estimate = estimate
source-codex-log = Codex log

projection-full-in = full in { DUR($ms) }
projection-reset-first = resets first
status-busy = busy
status-idle = idle
status-unknown = unknown

data-from-app = Live data from the TokenRadar app.
data-from-db = The TokenRadar app is not running; showing the last stored values.
data-none = No TokenRadar data yet. Start the app once so it can index your sessions.
no-limits = No limit readings yet.
no-live-sessions = No running Claude Code sessions.
updated-ago = updated { DUR($ms) } ago
estimate-note = ≈ marks estimates, not measurements.
statusline-last-seen = Statusline bridge last seen { DUR($ms) } ago.
statusline-never-seen = Statusline bridge not seen yet. Install it with: tokenradar statusline install

## statusline install / uninstall

install-no-config-dir = Claude Code config directory not found: { $path }
install-already = The statusline bridge is already installed in { $path }.
install-confirm = Apply this change to { $path }? [y/N]
install-done = Installed. Backup: { $backup }
install-chained = Your previous statusline command keeps running; its output is shown above TokenRadar's line.
install-chain-hint = Your previous statusline command is replaced. Re-run with --chain to keep it.
install-previous-mangled = Note: Git Bash removes the backslashes in your previous statusline command, so it most likely prints nothing today: { $command }. Write its path with forward slashes (/), or install with --no-chain.
install-cancelled = Nothing changed.
install-not-interactive = Not a terminal; pass --yes to apply without asking.
uninstall-not-installed = The statusline bridge is not installed in { $path }.
uninstall-done = Removed. Backup: { $backup }
uninstall-restored = Your previous statusline setting was restored.
settings-changed-meanwhile = { $path } changed while waiting; nothing was written. Run the command again.
settings-invalid = { $path } is not a valid JSON object; nothing was changed.

## Errors and help

error = error: { $message }
cli-help =
    TokenRadar - usage limits for AI coding agents
    Usage: tokenradar <COMMAND> [OPTIONS]
    Commands:
      status [--json]                     Limits, live sessions and data sources
      limits [--json]                     Current limit readings
      sessions [--json]                   Running Claude Code sessions
      statusline [--chain]                Claude Code statusline (reads JSON from stdin)
      statusline install [--chain|--no-chain] [--yes] [--config-dir <DIR>]
      statusline uninstall [--yes] [--config-dir <DIR>]
    Options:
      -h, --help       Print help
      -V, --version    Print version

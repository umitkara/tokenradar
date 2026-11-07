# The `tokenradar` console binary and the Claude Code statusline.

## Statusline. Keep these short: the line shares one terminal row with Claude Code.

sl-label-five-hour = 5h
sl-label-seven-day = 7d
sl-percent = { PCT($pct) }
sl-resets-in = { DUR($ms, "compact") }
sl-eta = ▲{ DUR($ms, "compact") }
# How much of the window's time has passed (clock time, not usage).
sl-elapsed = ⏱{ PCT($pct) }
sl-session-share = ≈{ PCT($pct) } this session
# How full the context window is. `ctx` stays as the short form in every language.
sl-label-context = ctx
# What the session would cost at API prices (`cost.total_cost_usd`); not a bill. Keep the `API` mark.
sl-cost = API ${ NUM($usd, 2) }
sl-cost-precise = API ${ NUM($usd, 4) }
# Tokens in the context window now: input (with cache) and the last output.
sl-label-tokens-in = in
sl-label-tokens-out = out
# Session time, and the time spent waiting for the API.
sl-label-duration = time
sl-label-api-duration = api
sl-label-git = git
sl-duration = { DUR($ms, "compact") }
# Counts. `k` and `M` stay as the short forms in every language.
sl-number = { NUM($n, 0) }
sl-thousands = { NUM($n, $d) }k
sl-millions = { NUM($n, $d) }M
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
projection-reset-first = won't fill
status-busy = busy
status-idle = idle
status-unknown = unknown

data-from-app = Live data from the TokenRadar app.
data-from-db = The TokenRadar app is not running; showing the last stored values.
data-none = No TokenRadar data yet. Start the app once so it can index your sessions.
no-limits = No limit readings yet.
limit-absent = { $name }: not on this account.
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
uninstall-none = TokenRadar's statusline bridge is not installed anywhere.

## mod install / uninstall

mod-missing = The Claude Code mod is not next to this tokenradar; it comes with the installed app.
mod-already = The Claude Code mod is already listed in { $path }.
mod-done = Installed. Claude Code now loads the mod in every session. Backup: { $backup }
mod-restart = Restart Claude Code to load it. Do not also start it with --plugin-dir for this mod, or it loads twice.
mod-not-installed = The Claude Code mod is not listed in { $path }.
mod-none = TokenRadar's Claude Code mod is not installed anywhere.
settings-changed-meanwhile = { $path } changed while waiting; nothing was written. Run the command again.
settings-invalid = { $path } is not a valid JSON object; nothing was changed.

## path add / remove

path-added = Added { $path } to your user PATH. New terminals will find tokenradar.
path-already = { $path } is already on your user PATH.
path-removed = Removed { $path } from your user PATH.
path-absent = { $path } is not on your user PATH.
path-too-long = Your user PATH would be { $length } characters long, over the { $max } that Windows tools handle safely; nothing was changed. Add { $path } by hand or shorten your PATH.
path-unsupported = This command is only available on Windows.

## Errors and help

doctor-done = Diagnostics bundle written: { $path }
    It holds no token, conversation content or readable path. Open it and check before sharing.
error = error: { $message }
cli-help =
    TokenRadar - usage limits for AI coding agents
    Usage: tokenradar <COMMAND> [OPTIONS]
    Commands:
      status [--json]                     Limits, live sessions and data sources
      limits [--json]                     Current limit readings
      sessions [--json]                   Running Claude Code sessions
      history [--json]                    Readings of the current windows, for charts (JSON)
      statusline [--chain]                Claude Code statusline (reads JSON from stdin)
      statusline install [--chain|--no-chain] [--yes] [--config-dir <DIR>]
      statusline uninstall [--yes] [--config-dir <DIR> | --all]
      mod install [--yes] [--config-dir <DIR>]
                                          Load TokenRadar's Claude Code mod in every session
      mod uninstall [--yes] [--config-dir <DIR> | --all]
      doctor [--out <ZIP>]                Diagnostics bundle to share when reporting a problem
      path add|remove                     Put this tokenradar on your user PATH, or take it off
    Options:
      -h, --help       Print help
      -V, --version    Print version

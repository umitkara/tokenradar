# The `tokenradar` console binary and the Claude Code statusline.

## Statusline. Keep these short: the line shares one terminal row with Claude Code.

sl-label-five-hour = 5h
sl-label-seven-day = 7d
sl-percent = { PCT($pct) }
sl-resets-in = { DUR($ms, "compact") }
sl-eta = ▲{ DUR($ms, "compact") }
# How much of the window's time has passed (clock time, not usage). $mark is a symbol
# (`⏱`, or an hourglass icon), the same in every language.
sl-elapsed = { $mark }{ PCT($pct) }
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
# Step 12m. What is left of a limit or the context window.
sl-remaining = { PCT($pct) } left
# $n is a count already formatted (`185k`).
sl-tokens-left = { $n } left
# The weekly window's room left per day until its reset.
sl-daily-budget = { PCT($pct) }/day
# The session's API-priced cost per hour; keep the `API` mark.
sl-burn-rate = API ${ NUM($usd, 2) }/h
# The share of the last request's input read from the prompt cache.
sl-label-cache = cache
# A reset as a local time: `Mon 09:00`.
sl-reset-clock = { $day } { $time }
sl-weekday-mon = Mon
sl-weekday-tue = Tue
sl-weekday-wed = Wed
sl-weekday-thu = Thu
sl-weekday-fri = Fri
sl-weekday-sat = Sat
sl-weekday-sun = Sun
# Step 12n. How many times the session was compacted. $mark is a symbol (`⟲`, or a fold
# icon), the same in every language.
sl-compact-total = { $mark }{ NUM($n, 0) }
sl-compact-split = { $mark }{ NUM($auto, 0) } auto { NUM($manual, 0) } manual
# Compactions with a trigger this version does not know.
sl-compact-other = { NUM($n, 0) } other
# Auto-compact is turned off in Claude Code's settings.
sl-compact-off = auto off
# $count and $limit are already formatted (`⟲ 3`, `300k`).
sl-compact-with-limit = { $count } · { $limit }
# The setting alone, while the count is not known.
sl-compact-limit-only = { $mark } { $limit }
# Step 12q. The session's reasoning effort, as Claude Code names the levels.
sl-effort-low = low
sl-effort-medium = medium
sl-effort-high = high
sl-effort-xhigh = xhigh
sl-effort-max = max
# Fast mode is on; extended thinking is off.
sl-effort-fast = fast
sl-effort-no-thinking = no thinking
# $model and $effort are already formatted (`Opus 5.5`, `high`).
sl-model-with-effort = { $model } · { $effort }
# When the prompt cache goes cold, as a local time ($time is `14:32`).
sl-cache-until = →{ $time }
# Warm, with neither the time nor the cache lifetime known.
sl-cache-warm = warm
sl-cache-cold = cold
# Requests that missed the prompt cache this session.
sl-cache-misses = { NUM($n, 0) } { $n ->
    [one] miss
   *[other] misses
}
# The likely cause of the last miss, as Claude Code reports it.
sl-miss-tools-changed = tools changed
sl-miss-system-prompt-changed = system prompt changed
sl-miss-ttl-expired-5m = 5m cache expired
sl-miss-likely-server-side = likely server side
# The whole session's cache ratio, next to the last request's.
sl-cache-session = { PCT($pct) } (session)
# The tokens the next request writes to the cache again if it has gone cold. $n is `45k`.
sl-recache = if cold { $n }
# The spend limit behind a Claude apps gateway. `$` stays in every language.
sl-label-spend = $
sl-spend-amount = ${ NUM($used, 0) }/${ NUM($limit, 0) }
# The open pull request's review state.
sl-pr-approved = approved
sl-pr-changes-requested = changes requested
sl-pr-pending = pending
sl-pr-draft = draft
# A linked git worktree.
sl-label-worktree = wt
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
# $models is a comma-separated list of model ids.
doctor-unpriced = No price yet for { $models }. Cost is estimated until TokenRadar is updated.
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

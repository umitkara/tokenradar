# The tray panel (Step 10). Empty and error states follow design-language.md section 9.

panel-live-sessions = Running sessions
panel-top-sessions = Largest this window
panel-no-live-sessions = No session is running right now.
panel-open-settings = Settings
panel-close = Close
# Running Claude Code sessions, in the header.
panel-active = { $count } active
# $plan is a plan name such as "Max 20x"; $label is the account, e.g. "Claude".
panel-account = { $label } · { $plan }
# More rows than the list shows.
panel-more = +{ $count } more
panel-limits = Limits
panel-sessions = Sessions
panel-loading = Loading usage…
panel-extra-usage = Extra usage is on: usage past a limit is billed separately.
# The account has no 5-hour window (verified Ultra accounts, for example); only weekly limits apply.
panel-no-five-hour = This account has no 5-hour window. Only the weekly limits apply.

## Projection under the limits. $kind is "weekly" for the weekly limit, else "five_hour".

# $eta is the time until 100 %, $reset the time until the window resets; $approx is "yes" when
# the projection rests on few readings.
panel-projection-will-hit = { $kind ->
    [weekly] ▲ Weekly: at this rate, full in { $approx ->
        [yes] ≈{ DUR($eta) }
       *[no] { DUR($eta) }
    }, resets in { DUR($reset) }
   *[five_hour] ▲ At this rate, full in { $approx ->
        [yes] ≈{ DUR($eta) }
       *[no] { DUR($eta) }
    }, resets in { DUR($reset) }
}
panel-projection-at-reset = { $kind ->
    [weekly] Weekly: at this rate ≈{ PCT($pct) } by the reset
   *[five_hour] At this rate ≈{ PCT($pct) } by the reset
}
panel-projection-idle = No usage right now
# Shown as the projection's tooltip when it rests on few readings.
panel-projection-low = Few readings so far; a rough estimate.

## Footer

panel-refresh = Refresh
panel-refresh-wait = Refreshed less than a minute ago
panel-refresh-blocked = Refresh is paused until the API is reachable again
panel-mute = Mute notifications for 1 hour
panel-unmute = Unmute notifications

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
# $models is a comma-separated list of model ids.
panel-unpriced = No price yet for { $models }. Cost is estimated until TokenRadar is updated.
panel-diagnostics = Show diagnostics

## The panel's two views (ADR-048)

panel-views = Panel view
panel-view-summary = Summary
panel-view-detail = Details

## The detail view: each limit against the time, the weekly time strip, the usage chart

detail-loading = Loading details…
# $elapsed is the share of the window that has passed; $points how far use is from it.
# $dir is "ahead", "behind" or "even".
detail-pace = Time gone { PCT($elapsed) } · { $dir ->
    [ahead] { NUM($points, 0) } points ahead of the time line
    [behind] { NUM($points, 0) } points behind the time line
   *[even] in step with the time line
}
# $name is a limit name such as "Weekly limit".
detail-strip-title = { $name } · time gone { PCT($elapsed) }
detail-chart-title = { $name } over the window
detail-now = now
detail-legend-midnight = midnight
detail-legend-period = every 24 hours from the reset, with the time line's value
detail-legend-time = time line
detail-legend-use = use
detail-chart-empty = No stored readings of this window yet.
detail-chart-alt = { $name } over the window against the time line: { PCT($used) } used, { PCT($elapsed) } of the time gone.
# What the session cost at API prices in this window; not a bill. Keep the `API` mark.
detail-session-cost = API ${ NUM($usd, 2) }
detail-session-cost-tip = API-equivalent cost in this window, not a bill.
detail-estimate-note = ≈ marks an estimate, not a measurement.
# The learned usage-to-limit rate behind a cost-based projection (Step 13).
# $level is high, medium or low.
detail-calibration = Learned rate: { $level ->
        [high] high
        [medium] medium
       *[low] low
    } confidence, from { $intervals } intervals.
# A recent change in how much of the limit a dollar of use takes. $dir is up or down.
detail-calibration-drift = { $dir ->
        [up] Use now counts more toward this limit than before; relearning.
       *[down] Use now counts less toward this limit than before; relearning.
    }

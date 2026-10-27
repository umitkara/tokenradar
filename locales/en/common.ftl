# Shared by every window: limit names, values, sources, sessions and component text.

app-name = TokenRadar

## Limits. The outer ring and top bar are always weekly; the inner ring and bottom bar are the
## 5-hour window.

limit-five-hour = 5-hour window
limit-five-hour-short = 5h
limit-weekly = Weekly limit
limit-weekly-short = 7d
# A weekly limit of one model. $name is the model, e.g. "Fable".
limit-weekly-scoped = { $name } · weekly

# A percentage; estimates always carry "≈".
value-pct = { $estimate ->
    [yes] ≈{ PCT($pct) }
   *[no] { PCT($pct) }
}
value-none = –

resets-in = Resets in { DUR($ms) }
# Under the ring's center readout, where "Resets in" does not fit: just the time left.
resets-in-short = { DUR($ms) }
# $at is a weekday and time, e.g. "Wed 10:59".
resets-at = Resets { $at }
resets-now = Resetting now
projection-full-in = At this rate, full in { DUR($ms) }
projection-at-reset = { PCT($pct) } projected at reset

## Screen reader sentences for the ring and bars (design-language.md section 7).

aria-limit = { $estimate ->
    [yes] { $name } about { PCT($pct) } used (estimate)
   *[no] { $name } { PCT($pct) } used
}
aria-limit-resets = { $estimate ->
    [yes] { $name } about { PCT($pct) } used (estimate), resets in { DUR($ms, "long") }
   *[no] { $name } { PCT($pct) } used, resets in { DUR($ms, "long") }
}
aria-limit-none = { $name }: no data yet
aria-ring = { $outer }. { $inner }.

severity-warn = Warning
severity-crit = Critical

## Where a number came from.

source-statusline = Live
source-api = API
source-estimate = Estimated
source-codex-log = Codex log
source-updated = Updated { DUR($ms) } ago
source-updated-now = Updated just now

## Sessions

session-busy = Busy
session-idle = Idle
session-unknown = Unknown
session-untitled = Untitled session
# A session's estimated share of the current 5-hour window.
session-share = ≈{ PCT($pct) } this window
# $k is thousands of tokens per minute.
session-rate = { NUM($k, 1) }k tok/min

## Generic states

loading = Loading…
no-limit-data = No limit data yet

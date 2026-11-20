# The main window (Step 15): navigation, page titles and the pages' empty states.

main-nav = Pages
main-nav-overview = Overview
main-nav-windows = Windows
main-nav-sessions = Sessions
main-nav-projects = Projects
main-nav-patterns = Patterns
main-nav-settings = Settings
# The window title: $page is the page's name.
main-title = { $page } – TokenRadar
main-loading = Loading…
main-load-failed = This page could not be loaded. Close the window and open it again.

## Pages that arrive in a later version.

main-soon-windows = The limit window archive is coming
main-soon-windows-body = Each closed 5-hour and weekly window, how full it got and what used it.
main-soon-sessions = The session list is coming
main-soon-sessions-body = Every Claude Code session with its cost and its share of your limits.
main-soon-projects = The project list is coming
main-soon-projects-body = Your projects with their sessions, cost and share of your limits.
main-soon-patterns = Usage patterns are coming
main-soon-patterns-body = When in the week you use Claude Code most, as a heat map.

## Charts

main-chart-7d = Last 7 days
# Screen reader summary of a percentage chart. $name is the limit, $pct its latest value.
chart-alt = { $name }, last 7 days, latest { PCT($pct) }
chart-empty = No readings in this range yet.
# Legend and tooltip: a window that reached its limit.
chart-hit-limit = Limit reached
chart-spans = Sessions
chart-span-share = Share of the weekly limit
# Screen reader name of a session in the lane. $value is its share.
chart-span-alt = { $name }: { $value }

## Overview (Step 15b)

overview-now = Now
overview-calibration = Calibration: { $level ->
        [high] high
        [medium] medium
       *[low] low
    } confidence
overview-calibration-tip = How usage counts toward this limit, learned from { $n } intervals.
overview-calibration-none = No calibration yet
overview-estimates-only = Only estimates so far. Install the statusline bridge or allow API access for measured values.
# Explains the session lane under the chart.
overview-spans-note = The lines below are the 5 sessions with the largest share of the weekly limit. Hover one for its name and share.

overview-week = This week
# $at is the local day and time the weekly window started ("Wed 11:00").
overview-week-since = Week started { $at }
overview-week-rolling = Weekly window unknown; last 7 days
overview-cost = API-equivalent cost
overview-cost-value = API ${ NUM($usd, 2) }
overview-cost-tip = Computed at API prices; not a bill.
overview-change = { $dir ->
        [up] ▲ { PCT($pct) }
        [down] ▼ { PCT($pct) }
       *[flat] No change
    }
overview-change-basis = against the same point last week
overview-change-none = No previous week to compare with
overview-previous = All of last week: API ${ NUM($usd, 2) }
overview-tokens = Tokens
overview-token-input = Input
overview-token-output = Output
overview-token-cache-read = Cache read
overview-token-cache-write = Cache write
# A shortened count: $unit is its magnitude.
overview-count = { $unit ->
        [k] { NUM($n, 1) }K
        [m] { NUM($n, 1) }M
        [b] { NUM($n, 1) }B
       *[one] { NUM($n, 0) }
    }
overview-models = Model mix
overview-model-rest = Other models
overview-week-empty = No usage this week yet.

overview-top = Top consumers
overview-top-sessions = Sessions
overview-top-projects = Projects
overview-top-share = Share of the weekly limit
overview-top-cost = API-equivalent cost this week
overview-top-trend = Cost by day of this week
overview-learning = Shares are split by cost for now and are approximate. They settle once calibration reaches medium confidence. Now { $level ->
        [high] high
        [medium] medium
       *[low] low
    } confidence, { $n } intervals.
overview-learning-none = Shares are split by cost for now and are approximate. They settle once calibration is learned.
overview-learning-no-week = No weekly window is known, so there are no shares; ranked by cost.
overview-stale = Shares were computed with an older calibration; they are being redone in the background.

overview-other = Other usage
overview-other-body = Usage outside Claude Code on this computer: claude.ai, Desktop, Cowork or another computer.
overview-other-ours = Our estimate
overview-other-ours-value = { PCT($pct) } of the weekly limit
overview-other-ours-share = { PCT($pct) } of all usage
overview-other-api = Anthropic's breakdown
overview-other-none = Our own estimate is computed once calibration reaches medium confidence.
overview-other-no-breakdown = The breakdown only arrives while the usage API is on.
overview-other-suspect = Our estimate is more than 10 points below the breakdown. Calibration may count usage on this computer too high.
overview-other-excess = Our estimate is above the breakdown. Claude Code on another computer also shows here as "Other".
overview-surface-claude-code = Claude Code
overview-surface-chat = claude.ai
overview-surface-cowork = Cowork
overview-surface-other = Other

overview-plan = Plan value
overview-plan-month = API equivalent this month
overview-plan-fee = { $source ->
        [user] Plan fee (yours)
       *[table] Plan fee ({ $plan })
    }
overview-plan-usd = ${ NUM($usd, 2) }
overview-plan-ratio = ×{ NUM($x, 1) }
overview-plan-ratio-label = Times the fee
overview-plan-note = These numbers are for information: computed at API prices, not your bill.
overview-plan-no-fee = This plan's fee is not known. You can enter it in the settings.
overview-plan-elapsed = { PCT($pct) } of the month gone
overview-plan-previous = Last month ×{ NUM($x, 1) } (API ${ NUM($usd, 2) })
overview-plan-edit = Edit plan fee

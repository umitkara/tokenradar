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

## Sessions (Step 16a)

sessions-filters = Filters
sessions-search = Search sessions
sessions-search-placeholder = Name, project or session ID
sessions-range = Time range
sessions-range-today = Today
sessions-range-7d = 7 days
sessions-range-30d = 30 days
sessions-range-all = All
sessions-range-custom = Custom
sessions-from = From
sessions-to = To
sessions-project = Project
sessions-project-all = All projects
sessions-model = Model
sessions-model-all = All models
sessions-live = Running only
sessions-sort = Sort by
sessions-sort-last-active = Last active
sessions-sort-started = Started
sessions-sort-cost = Cost
sessions-sort-tokens = Tokens
sessions-sort-impact = Impact
sessions-list = Sessions
sessions-col-name = Session
sessions-col-start = Started
sessions-col-duration = Duration
sessions-col-models = Models
sessions-col-cost = Cost
sessions-col-tokens = Tokens
sessions-col-impact = Impact
sessions-col-subagents = Subagents
sessions-running = Running
sessions-ago = { DUR($ms) } ago
sessions-just-now = Just now
sessions-usd = ${ NUM($usd, 2) }
sessions-impact-weekly = Share of the weekly limit in this range. Split from usage, so it is an estimate.
sessions-impact-session = Share of 5-hour windows in this range, summed.
sessions-impact-dim = Low confidence: calibration is still learning.
sessions-deleted = Transcript deleted
sessions-deleted-tip = Claude Code deleted this session's transcript. The totals recorded before remain.
sessions-loading-more = Loading more sessions…
sessions-empty = No sessions in this range
sessions-empty-body = Sessions show up here as soon as Claude Code writes usage.
sessions-empty-filtered = No session matches these filters
sessions-empty-filtered-body = Try a longer time range or clear the search.
sessions-clear = Clear filters

## Session detail (Step 16a)

session-back = Back to sessions
session-breadcrumb = Breadcrumb
session-not-found = This session is not in the database any more.
# $level is Claude Code's /effort value; unknown values are shown as they are.
session-effort = Effort: { $level ->
        [low] low
        [medium] medium
        [high] high
        [xhigh] extra high
       *[other] { $level }
    }
session-fast = Fast mode
session-fast-tip = At least one request in this session ran in fast mode.
session-chart = Cost over time
# $ms is how long one bar is.
session-chart-bucket = One bar is { DUR($ms) }
# $total is the formatted cost ("$4.20").
session-chart-alt = Cost of this session over time, stacked by model family. Total { $total }.
session-chart-empty = No cost recorded for this session.
session-chart-other = Other models
session-chart-window = Limit window
session-impact = Share of limits
session-impact-window = Window
session-impact-share = Share
session-impact-peak = Peak
session-impact-hit = Hit the limit
session-impact-yes = Yes
session-impact-no = No
session-impact-unknown = Unknown
session-impact-estimated = Estimated window, no readings.
session-impact-empty = This session touched no known limit window.
session-impact-note = Shares are split from usage and marked ≈; dim values have low confidence.
session-subagents = Subagents
session-subagents-empty = No subagents.
session-subagent-untitled = Untitled subagent
session-cache = Cache
session-cache-hit = Hit ratio
session-cache-write-5m = 5 min cache write
session-cache-write-1h = 1 h cache write
session-cache-savings = Saved
session-cache-savings-value = ≈${ NUM($usd, 2) }
session-cache-savings-tip = What the cache reads would have cost as plain input, at API prices, minus what they cost.
session-meta = Details
session-branch = Git branch
session-version = Claude Code version
session-entrypoint = Started from
session-dirs = Folders
session-dirs-none = No folder recorded. Folders and effort are only known for sessions whose transcript was still on disk when this version first read it.
session-open-dir = Open folder
session-dir-missing = This folder no longer exists.
session-dir-failed = The folder could not be opened.
session-tokens = Tokens

## Projects (Step 16b)

projects-search = Search projects
projects-search-placeholder = Name or part of the path
projects-group = Group by folder
projects-hide-temp = Hide temporary folders
projects-sort-last-active = Last active
projects-sort-cost = Cost
projects-sort-impact = Weekly impact
projects-sort-sessions = Sessions
projects-hidden = { $n ->
        [one] 1 project in a temporary folder is hidden.
       *[other] { NUM($n, 0) } projects in temporary folders are hidden.
    }
projects-show-temp = Show them
projects-list = Projects
projects-tree = Projects by folder
projects-col-name = Project
projects-col-path = Path
projects-col-sessions = Sessions
projects-col-last = Last active
projects-col-cost = Cost
projects-col-weekly = Weekly
projects-col-trend = 8 weeks
projects-weekly-tip = Share of weekly limits in this range, summed over the project's sessions. Split from usage, so it is an estimate.
projects-trend-tip = Cost per week over the last 8 weeks, ${ NUM($usd, 2) } in total
projects-count = { $n ->
        [one] 1 project
       *[other] { NUM($n, 0) } projects
    }
projects-expanded = expanded
projects-collapsed = collapsed
projects-loading = Loading projects…
projects-empty = No projects in this range
projects-empty-body = Projects show up here as soon as Claude Code writes usage in a folder.
projects-empty-filtered = No project matches these filters
projects-empty-filtered-body = Try a longer time range, clear the search or show temporary folders.

## Project detail (Step 16b)

project-back = Back to projects
project-not-found = This project is not in the database any more.
project-cost = Cost, all time
project-weekly-total = Weekly impact, all time
project-weeks = Weekly impact
project-weeks-empty = No weekly window recorded yet.
project-weeks-note = Share of each weekly window, split from usage (≈). Dashed bars are estimated windows.
project-weeks-by-cost = No shares yet: calibration is still learning. The bars show each week's cost.
project-week-estimated = estimated window
project-models-empty = No usage recorded.
project-top = Most expensive sessions
project-top-empty = No sessions.
project-sessions = Sessions

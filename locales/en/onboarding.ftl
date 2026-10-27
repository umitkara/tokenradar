# The first-run window (Step 11c): four steps. The window also uses settings.ftl for the
# statusline bridge, the API consent and the folder picker.

onboarding-steps = Setup steps
# $n is the current step, $total the number of steps.
onboarding-step = Step { NUM($n, 0) } of { NUM($total, 0) }
onboarding-back = Back
onboarding-next = Next
onboarding-skip = Skip
onboarding-finish = Finish

## 1. Welcome

onb-welcome-title = Welcome to TokenRadar
onb-welcome-pitch = See your Claude Code limits from the taskbar: how full each window is, when it resets, and which sessions are using it.
onb-welcome-local = Everything stays on this computer. No account, no telemetry.

## 2. Claude Code

onb-claude-title = Claude Code
onb-claude-found = Claude Code was found.
onb-claude-indexing = Reading your sessions… { PCT($pct) }
onb-claude-recent-ready = The current window and this week are ready.
onb-claude-indexed = All sessions are read.
onb-claude-waiting = Reading starts in a moment.
onb-claude-more-dirs = { NUM($n, 0) } more { $n ->
    [one] folder is
   *[other] folders are
} watched too.
onb-claude-not-found = Claude Code was not found.
onb-claude-not-found-hint = TokenRadar looks in .claude in your user folder and in CLAUDE_CONFIG_DIR. If Claude Code keeps its data somewhere else, choose that folder.

## 3. Limit data

onb-limits-title = Limit data
onb-limits-intro = Claude Code does not save its limit percentages in its logs. TokenRadar can get them in two ways; you can use either, both or neither.
onb-limits-api-optional = Optional
onb-limits-api-show = Read how it works
onb-limits-ready-statusline = Limits come from the statusline bridge.
onb-limits-ready-api = Limits come from the usage API.
onb-limits-skipped = Without either, limit percentages are not shown; TokenRadar only tracks tokens and cost. You can turn both on later in Settings › Data sources.

## 4. Tray

onb-tray-title = Find TokenRadar in the tray
onb-tray-intro = TokenRadar lives in the taskbar's notification area. Windows 11 puts new icons under the ^ arrow first.
onb-tray-figure = The taskbar with the notification area opened under the ^ arrow; the TokenRadar ring is inside.
onb-tray-show = Always show the icon
onb-tray-shown = The icon is on the taskbar.
onb-tray-not-done = Windows did not let TokenRadar move the icon.
onb-tray-manual = Open Settings › Personalization › Taskbar › Other system tray icons and turn TokenRadar on.
onb-tray-manual-win10 = Open Settings › Personalization › Taskbar › Select which icons appear on the taskbar and turn TokenRadar on.
onb-tray-open-settings = Open taskbar settings
onb-tray-autostart = Start with Windows
onb-tray-autostart-hint = Limits are tracked only while TokenRadar runs.
onb-tray-done-hint = Click the icon for the panel; right-click it for settings and this guide.

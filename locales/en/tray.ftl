# Tray icon: tooltip and context menu.

## Tooltip. Windows cuts it at 127 characters; the app falls back to the short forms when the
## full text is longer.

tray-title = TokenRadar
# A limit's percentage; estimates carry "≈".
tray-value = { $estimate ->
    [yes] ≈{ PCT($pct) }
   *[no] { PCT($pct) }
}
tray-five-hour = 5h: { $value }
tray-five-hour-resets = 5h: { $value } · resets in { DUR($ms) }
tray-five-hour-resets-short = 5h: { $value } ({ DUR($ms) })
tray-weekly = Weekly: { $value }
# $at is a weekday and time, e.g. "Wed 10:59".
tray-weekly-resets = Weekly: { $value } · resets { $at }
tray-weekly-resets-short = Weekly: { $value } ({ $at })
# A weekly limit of one model, shown from 75 %. $name is the model, e.g. "Fable".
tray-weekly-scoped = { $name } · weekly: { $value }
tray-weekly-scoped-resets = { $name } · weekly: { $value } · resets { $at }
tray-weekly-scoped-short = { $name } · 7d: { $value }
tray-no-data = No limit data yet
tray-stale = Last data: { $at }
tray-error = Can't read limit data
tray-error-short = Can't read data
# $until is a time, e.g. "10:59".
tray-muted = Notifications muted until { $until }
tray-muted-short = Muted until { $until }

## Context menu

tray-menu-open-panel = Open panel
tray-menu-mute = Mute notifications for 1 hour
tray-menu-unmute = Unmute notifications
tray-menu-settings = Settings…
tray-menu-onboarding = Getting started…
tray-menu-about = About TokenRadar
# Development builds only.
tray-menu-gallery = Component gallery (development)
tray-menu-quit = Quit

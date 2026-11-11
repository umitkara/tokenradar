# The settings window (Step 11b): five tabs, the statusline dialog and the delete dialog.

settings-title = Settings
settings-tabs = Settings sections
tab-general = General
tab-sources = Data sources
tab-statusline = Statusline
tab-notifications = Notifications
tab-privacy = Privacy & data
tab-about = About

settings-save-failed = The setting could not be saved. Try again.
settings-cancel = Cancel
settings-close = Close
# Language names are written in their own language in both files.
language-en = English
language-tr = Türkçe

## Sizes. The window picks the unit; $n is the value in that unit.

size-bytes = { NUM($n, 0) } bytes
size-kb = { NUM($n, 0) } KB
size-mb = { NUM($n, 1) } MB
size-gb = { NUM($n, 2) } GB

## General

general-language = Language
general-language-system = Same as Windows
general-theme = Theme
general-theme-system = Same as Windows
general-theme-light = Light
general-theme-dark = Dark
general-autostart = Start with Windows
general-autostart-hint = TokenRadar starts in the tray when you sign in.
general-autostart-unknown = The startup setting could not be read.
general-warm = Keep the closed panel ready
general-warm-hint = A closed panel stays in memory this long, so it opens instantly. Shorter uses less memory.
general-warm-off = Free it at once
general-warm-option = { DUR($ms) }

## General: the desktop widget (Step 18)

general-widget = Desktop widget
general-widget-enabled = Show the widget
general-widget-enabled-hint = A small strip with the limit bars that stays on top. Drag it anywhere; double-click opens the panel, right-click shows its menu.
general-widget-opacity = Background opacity
general-widget-opacity-option = { PCT($pct) }
general-widget-pace = Time mark
general-widget-pace-hint = A mark on each bar shows how much of the window's time has passed.
general-widget-fullscreen = Hide during full-screen apps
general-widget-fullscreen-hint = Games, videos and presentations are not covered.
general-widget-click-through = Click-through
# $hotkey is the shortcut, e.g. "Ctrl+Alt+Shift+R".
general-widget-click-through-hint = Clicks go to the windows below. { $hotkey } or the tray menu makes the widget clickable again.
general-widget-hotkey = Click-through shortcut
general-widget-hotkey-hint = Works anywhere while the widget is shown. Click, then press the keys.
general-widget-hotkey-recording = Press the keys…
general-widget-hotkey-invalid = Use Ctrl, Alt or Win with a letter, a digit or F1–F24. Esc cancels.

## General: the widget's taskbar mode (Step 18b)

general-widget-mode = Position
general-widget-mode-floating = Floating
general-widget-mode-taskbar = On the taskbar
general-widget-mode-taskbar-hint = The strip sits on the taskbar, left of the notification area. Where the taskbar is on a side or at the top, hides automatically or has no room, the widget floats instead.
general-widget-align = Align
general-widget-align-tray = Next to the notification area
general-widget-align-left = At the left end
general-widget-offset = Distance
general-widget-offset-hint = How far the strip keeps from where it is aligned. Dragging it along the taskbar sets this too.
# $px is a distance in logical pixels.
general-widget-offset-option = { $px } px
general-widget-fullscreen-docked-hint = On the taskbar the widget always hides during full-screen apps.

## Data sources: Claude Code folders

sources-dirs = Claude Code folders
sources-dirs-hint = TokenRadar reads usage records from these folders. It never changes them.
sources-dir-primary = Primary
sources-dir-chosen = Chosen by you
sources-dir-env = From CLAUDE_CONFIG_DIR
sources-dir-stats = { NUM($sessions, 0) } { $sessions ->
    [one] session
   *[other] sessions
} · { NUM($projects, 0) } { $projects ->
    [one] project
   *[other] projects
} · { NUM($files, 0) } { $files ->
    [one] file
   *[other] files
}
sources-dir-last = Last activity { DUR($ms) } ago
sources-dir-version = Claude Code { $version }
sources-dir-pending = Not indexed yet.
sources-dir-none = No Claude Code folder was found. Choose one, or set CLAUDE_CONFIG_DIR before starting TokenRadar.
sources-dir-missing = The folder you chose no longer exists, so the detected folders are used.
sources-change = Choose folder…
sources-reset-dir = Use the detected folder
sources-change-note = The folder you choose becomes the primary one: the panel shows its limits. The other folders stay watched.
sources-pick-not-dir = That is not a folder.
sources-pick-not-claude = That folder has no projects folder and no settings.json. Choose the Claude Code folder (usually .claude in your user folder).
sources-pick-failed = The folder could not be used. Try again.

## Data sources: statusline bridge

statusline-title = Statusline bridge
statusline-hint = Claude Code hands its live limit percentages to the statusline command. With the bridge installed, TokenRadar shows those measured numbers instead of estimates.
statusline-recommended = Recommended
statusline-no-directory = Choose a Claude Code folder first.
statusline-not-installed = Not installed.
statusline-current = Your current statusline command:
statusline-installed = Installed.
statusline-installed-chained = Installed. Your previous command still runs:
statusline-installed-elsewhere = TokenRadar is the statusline, but it was not installed from here. Remove it the same way it was added.
statusline-last-seen = Last reading { DUR($ms) } ago
statusline-never-seen = No reading yet. Claude Code sends one with its next reply.
statusline-install = Install…
statusline-uninstall = Remove…
statusline-cli-missing = The tokenradar command was not found next to the app, so the bridge cannot be installed from here.

statusline-dialog-install = Install the statusline bridge
statusline-dialog-uninstall = Remove the statusline bridge
statusline-dialog-intro = TokenRadar will change the file below. Nothing is written until you confirm, and a backup is made first.
statusline-diff = Changes
statusline-chain = Keep my current statusline command
statusline-chain-hint = Your line stays, and TokenRadar's is added next to it.
statusline-chain-note = Your command keeps doing whatever it does today, including its own network requests. A script that calls the usage API, for example, keeps calling it.
statusline-mangled = Claude Code may run this command through Git Bash, which drops backslashes. Check that your line still appears after installing.
statusline-restores = Your previous statusline command comes back.
statusline-confirm-install = Install
statusline-confirm-uninstall = Remove
statusline-done = Done. Backup of the old file: { $path }
statusline-already = The bridge is already installed.
statusline-nothing = The bridge is not installed, so there is nothing to remove.
statusline-error-no-directory = No Claude Code folder was found.
statusline-error-cli-missing = The tokenradar command was not found next to the app.
statusline-error-invalid-settings = settings.json is not valid JSON, so it was left alone. Fix it in an editor and try again.
statusline-error-changed-meanwhile = settings.json changed while this was open. Nothing was written; review the new changes.
statusline-error-stale-plan = These changes are out of date. Nothing was written; review them again.
statusline-error-io = The file could not be written. Nothing was changed.

## Data sources: Claude Code mod

mod-title = Panel inside Claude Code
mod-experimental = Experimental
mod-hint = A side panel with your limits, a warning band above the prompt, and the /radar command, in every Claude Code session. Claude Code loads it from the app's folder; nothing else is downloaded.
mod-installed = Installed. Claude Code loads it from:
mod-missing = The mod's files are not next to the app, so it cannot be installed from here.
mod-dialog-install = Install the Claude Code mod
mod-dialog-uninstall = Remove the Claude Code mod
mod-dialog-restart = Restart Claude Code afterwards. If you started it with --plugin-dir for this mod before, stop doing that, or the mod loads twice.
mod-kept = { $count ->
    [one] One other plugin folder in the list stays.
   *[other] { $count } other plugin folders in the list stay.
}
mod-already = The mod is already installed.
mod-nothing = The mod is not installed, so there is nothing to remove.
mod-done-install = Installed. Restart Claude Code to load it.
mod-done-uninstall = Removed. Restart Claude Code to unload it.
mod-backup = Backup of the old file: { $path }

## Statusline editor (Step 12j)

statusline-look-moved = Choose what the line shows and how it looks on its own page.
statusline-edit-look = Edit appearance →
statusline-editor-title = Statusline
statusline-autosave = Changes are saved automatically.
statusline-editor-not-installed = The bridge is not installed, so Claude Code does not show this line yet.
statusline-editor-to-sources = Install it in Data sources
statusline-refresh = Live refresh
statusline-refresh-off = Off
statusline-refresh-every = { $n } s
statusline-refresh-cost = TokenRadar runs every { $n } seconds while a session is open, even when idle.
statusline-refresh-off-hint = The line is redrawn only after a message. Countdowns stand still in between.
statusline-refresh-differs = settings.json says { $n } s; Claude Code follows the file.
statusline-refresh-differs-off = settings.json has no live refresh; Claude Code follows the file.
statusline-refresh-saved = Saved.
statusline-dialog-refresh = Change live refresh
statusline-confirm-refresh = Change
statusline-refresh-chain-note = Your chained command runs on every refresh too.
statusline-undo = Undo
statusline-undo-hint = Undo the last change (Ctrl+Z)
statusline-restore-default = Restore default…
statusline-theme = Theme
statusline-theme-plain = Plain
statusline-theme-powerline = Powerline
statusline-theme-powerline-round = Rounded
statusline-colors = Colors
statusline-color-mode-auto = Auto
statusline-color-mode-always = Always
statusline-color-mode-never = Off
statusline-icons = Icons
statusline-icons-text = Text
statusline-icons-unicode = Unicode
statusline-icons-nerd = Nerd Font
statusline-icons-suggest = Powerline needs a Nerd Font. The same font can draw icons in place of the labels.
statusline-icons-use-nerd = Use Nerd Font icons
statusline-icons-no-font = Nerd Font icons need a Nerd Font, and none is installed. With other fonts they show as boxes, here and in Claude Code.
statusline-icons-not-mono = The installed Nerd Font is not a Mono variant, so an icon may spill into the next cell.
statusline-presets = Presets…
statusline-export = Export: copy the line as a [statusline] table
statusline-exported = Copied to the clipboard as a [statusline] table.
statusline-export-failed = The clipboard could not be written. Try again.
statusline-import = Import a [statusline] table…
statusline-preview = Preview with sample values
statusline-state = Sample state
statusline-state-normal = Normal
statusline-state-warn = Warning
statusline-state-crit = Critical
statusline-state-stale = Stale
statusline-width = Terminal width
statusline-columns = { $n } col
statusline-font-note = This theme needs a Nerd Font, and none is installed. With other fonts its glyphs show as boxes, here and in Claude Code.
statusline-margin = Right margin
statusline-margin-hint = Raise it if the line runs off the right edge in your real terminal.
statusline-reserved = The line keeps the last { $n } columns free.
statusline-align-note = In a real terminal, glyphs two cells wide can shift the alignment.
statusline-select-hint = Select an item to change it.

statusline-item-hidden = hidden
statusline-segment-absent = not on this account
statusline-reorder = Move { $name }: drag, or press ↑ and ↓
statusline-hide = Hide
statusline-show = Show
statusline-remove = Remove
statusline-add = Add item
statusline-line = Line { $n }
statusline-line-items = Items of line { $n }, in order
statusline-line-remove = Remove line { $n } and its items
statusline-line-removed = Line { $n } removed. Undo brings it back.
statusline-line-add = Add line
statusline-line-added = Line { $n } added.
statusline-line-empty = Empty line. Add an item or drag one here.
statusline-add-to-line = Add an item to line { $n }
statusline-moved-line = { $name } moved to line { $line }, position { $position } of { $count }.
statusline-move-up = Move up
statusline-move-down = Move down

statusline-add-used = in the line
statusline-add-full = The line holds at most { $max } items.
statusline-group-limit = Limits
statusline-group-session = Session
statusline-group-project = Project
statusline-group-layout = Layout

statusline-kind-five-hour = 5-hour window
statusline-kind-five-hour-hint = How much of the 5-hour window is used, with the reset countdown.
statusline-kind-seven-day = Weekly limit
statusline-kind-seven-day-hint = How much of the weekly limit is used, with the reset countdown.
statusline-kind-scoped = Model limits
statusline-kind-scoped-hint = Weekly limits of one model or surface, such as Fable's. Shown when Claude Code reports one.
statusline-kind-context = Context window
statusline-kind-context-hint = How full this session's context window is.
statusline-kind-session-share = This session's share
statusline-kind-session-share-hint = How much of the 5-hour window this session used. Hidden until TokenRadar knows.
statusline-kind-cost = API-equivalent cost
statusline-kind-cost-hint = What this session would cost at API prices. Not a bill, so the API mark stays.
statusline-kind-model = Model
statusline-kind-model-hint = The model this session runs on.
statusline-kind-tokens-in = Input tokens
statusline-kind-tokens-in-hint = Tokens sent to the model in this session.
statusline-kind-tokens-out = Output tokens
statusline-kind-tokens-out-hint = Tokens the model wrote in this session.
statusline-kind-duration = Session time
statusline-kind-duration-hint = How long this session has been running.
statusline-kind-api-duration = API time
statusline-kind-api-duration-hint = How long this session waited for the model.
statusline-kind-lines-changed = Lines changed
statusline-kind-lines-changed-hint = Lines added and removed in this session.
statusline-kind-vim-mode = Vim mode
statusline-kind-vim-mode-hint = The editor's Vim mode, when Vim mode is on.
statusline-kind-version = Claude Code version
statusline-kind-version-hint = The version of Claude Code running this session.
statusline-kind-agent-name = Agent
statusline-kind-agent-name-hint = The agent's name, when the session runs one.
statusline-kind-session-id = Session ID
statusline-kind-session-id-hint = Claude Code's ID for this session.
statusline-kind-cwd = Working folder
statusline-kind-cwd-hint = The folder Claude Code is working in.
statusline-kind-project-dir = Project folder
statusline-kind-project-dir-hint = The folder the session was started in.
statusline-kind-git-branch = Git branch
statusline-kind-git-branch-hint = The branch checked out in the project. Read from the repository, without running git.
statusline-kind-daily-budget = Daily budget
statusline-kind-daily-budget-hint = What is left of the weekly limit, shared over the days until its reset. Yellow when your use runs ahead of the time.
statusline-kind-burn-rate = Burn rate
statusline-kind-burn-rate-hint = This session's API-equivalent cost per hour. Shown after the first five minutes.
statusline-kind-session-name = Session name
statusline-kind-session-name-hint = The name of this Claude Code session, when it has one.
statusline-kind-cache-ratio = Cache ratio
statusline-kind-cache-ratio-hint = How much of the last request's input was read from the prompt cache.
statusline-kind-output-style = Output style
statusline-kind-output-style-hint = The output style this session uses.
statusline-kind-compact = Compactions
statusline-kind-compact-hint = How many times this session was compacted, and the auto-compact window when you set one in Claude Code's settings. The count needs the app running.
statusline-kind-effort = Effort
statusline-kind-effort-hint = The session's reasoning effort, as set with /effort. Words like "ultrathink" in one prompt and subagents' own effort do not change it. Hidden for models without effort levels.
statusline-kind-cache-timer = Cache timer
statusline-kind-cache-timer-hint = When the prompt cache goes cold if no request comes: as a local time, or as the time left. Requests after that write the cache again.
statusline-kind-cache-misses = Cache misses
statusline-kind-cache-misses-hint = How many requests in this session missed the prompt cache, and the likely cause of the last one.
statusline-kind-recache-cost = Recache cost
statusline-kind-recache-cost-hint = The tokens the next request writes to the cache again if it has gone cold. Hidden right after a compaction.
statusline-kind-spend-limit = Spend limit
statusline-kind-spend-limit-hint = How much of the spend limit is used. Only shown behind a Claude apps gateway that sets one; its dollars can be a few minutes behind.
statusline-kind-pr = Pull request
statusline-kind-pr-hint = The open pull request (or GitLab merge request) of the current branch, with its review state.
statusline-kind-worktree = Worktree
statusline-kind-worktree-hint = The linked git worktree the session works in. Hidden in the main working tree.
statusline-kind-repo = Repository
statusline-kind-repo-hint = The repository's owner and name, read from its origin remote.
statusline-kind-git-status = Git status
statusline-kind-git-status-hint = Staged (+), modified (~) and untracked (?) files, conflicts (!) and commits ahead of and behind the upstream. The app runs git for it, at most every few seconds; hidden while the app is not running.
statusline-kind-separator = Separator
statusline-kind-separator-hint = A glyph of your choice between items.
statusline-kind-line-break = Line break
statusline-kind-line-break-hint = Starts a new line. Items after it are shown below.
statusline-kind-custom-text = Custom text
statusline-kind-custom-text-hint = A fixed text of your choice.
statusline-kind-space = Space
statusline-kind-space-hint = Blank columns between two items.
statusline-kind-flex = Flexible space
statusline-kind-flex-hint = Fills the free width. Items after it go to the right; between two, to the middle.

statusline-absent-note = This account has no 5-hour window, so the item is left out of the line. It comes back if the window returns.
statusline-hidden-note = Hidden: kept with its settings, but not shown.
statusline-format = Format
statusline-format-bar = Bar
statusline-format-percent = Percent
statusline-format-fraction = Fraction
statusline-format-two-decimals = 0.00
statusline-format-four-decimals = 0.0000
statusline-format-short = Short
statusline-format-full = Full
statusline-format-compact = Compact
statusline-format-hms = h:mm:ss
statusline-format-both = Added and removed
statusline-format-net = Net
statusline-format-basename = Folder name
statusline-format-home = ~ path
statusline-format-remaining = Left
statusline-format-tokens-left = Tokens left
statusline-format-total = Total
statusline-format-split = Auto and manual
statusline-format-level = Level
statusline-format-flags = With fast and thinking
statusline-format-count = Count
statusline-format-cause = With cause
statusline-format-amount = Dollars
statusline-format-name = Name
statusline-format-host = With host
statusline-format-with-effort = With effort
statusline-format-last = Last request
statusline-format-session = Whole session
statusline-format-with-added = With added folders
statusline-format-clock = Clock
statusline-format-countdown = Countdown
statusline-format-dirty = Dirty mark
statusline-format-ahead-behind = Ahead and behind
statusline-untracked = Count untracked files
statusline-untracked-hint = Turn off for very large repositories: git skips the untracked scan and the line shows ?? instead of a number. The app also turns it off by itself when git is too slow.
statusline-countdown-frozen = Without live refresh the countdown stands still while the session is idle: Claude Code draws the line only after a message. Turn on live refresh, or use the clock.
statusline-label = Label
statusline-label-none = No label
statusline-label-hint = Leave it empty for the default label, shown faintly in the box.
statusline-glyph = Glyph
statusline-apply-all = Apply to all separators
statusline-apply-all-hint = Separators added later get this glyph too.
statusline-space-width = Width
statusline-flex-right = Items after it go to the right.
statusline-flex-center = The group between the two is centered.
statusline-flex-spread = The free width is shared between them.
statusline-text = Text
statusline-color = Color
statusline-color-auto = Auto
statusline-color-threshold-note = This is the color below 75%. At 75% the value turns yellow and at 90% red, whatever you choose here. Estimates and stale values stay gray.
statusline-color-black = Black
statusline-color-red = Red
statusline-color-green = Green
statusline-color-yellow = Yellow
statusline-color-blue = Blue
statusline-color-magenta = Magenta
statusline-color-cyan = Cyan
statusline-color-white = White
statusline-color-gray = Gray
statusline-color-bright-red = Bright red
statusline-color-bright-green = Bright green
statusline-color-bright-yellow = Bright yellow
statusline-color-bright-blue = Bright blue
statusline-color-bright-magenta = Bright magenta
statusline-color-bright-cyan = Bright cyan
statusline-color-bright-white = Bright white
statusline-reset-countdown = Reset countdown
statusline-reset-format = Reset as
statusline-reset-format-countdown = Countdown
statusline-reset-format-clock = Clock time
statusline-bar-width = Bar width
statusline-bar-style = Bar style
statusline-bar-style-blocks = Blocks
statusline-bar-style-thin = Thin
statusline-bar-style-dots = Dots
statusline-bar-style-capsule = Capsule
statusline-bar-style-solid = Solid
statusline-capsule-no-font = The capsule needs a Nerd Font, 3.4 or later, and none is installed. With other fonts it shows as boxes.
statusline-capsule-old-font = The capsule needs Nerd Fonts 3.4 or later. The installed font is older and draws boxes.
statusline-bar-in-blocks = Powerline blocks draw only the capsule, so this bar is left out.
statusline-pace = Time mark
statusline-pace-hint = How much of the window's time has passed: a mark on the bar and ⏱ after the value. Compare it with what you used.

statusline-presets-title = Presets
statusline-presets-intro = A preset replaces every item, the theme and the colors. Undo brings your line back.
statusline-presets-confirm = Apply
statusline-default-title = Restore the default line
statusline-default-intro = Your items, theme and colors are replaced by the default line. Undo brings them back.
statusline-default-confirm = Restore
statusline-preset-default = Default
statusline-preset-default-hint = Limits, context, this session's share and cost.
statusline-preset-minimal = Minimal
statusline-preset-minimal-hint = The limits and nothing else.
statusline-preset-detailed = Detailed
statusline-preset-detailed-hint = Limits and the session on the first line, the project on the second.
statusline-preset-two-line = Two lines
statusline-preset-two-line-hint = Limits first, the session below.
statusline-preset-powerline = Powerline
statusline-preset-powerline-hint = Colored blocks. Needs a Nerd Font.
statusline-preset-split = Split
statusline-preset-split-hint = Limits on the left; model, folder and branch on the right.

statusline-import-title = Import a statusline
statusline-import-intro = Paste a [statusline] table, for example one exported from another computer. Nothing changes until you apply it.
statusline-import-read = Read
statusline-import-error = This is not a valid TOML table. The parser said:
statusline-import-preview = What it looks like
statusline-import-ignored = { $count ->
    [one] One key was skipped:
   *[other] { $count } keys were skipped:
}
statusline-import-apply = Apply

## Data sources: usage API

api-title = Usage API
api-hint = Without the statusline bridge, TokenRadar can ask Anthropic for your limits instead. It stays off until you allow it.
api-what-label = What happens
api-what = Every few minutes TokenRadar asks api.anthropic.com for your current limit percentages: the same request Claude Code makes.
api-reads-label = What is read
api-reads = The access token in Claude Code's .credentials.json, only at the moment of a request. It is never stored, logged or shown.
api-never-label = What is never read
api-never = The refresh token. TokenRadar never renews or changes your sign-in. When the token expires, it waits until Claude Code renews it.
api-where-label = Where it goes
api-where = Only to api.anthropic.com, over HTTPS. Nothing else leaves your computer.
api-revoke = You can turn it off at any time; requests stop at once.
api-allow = Allow API access
api-turn-off = Turn off API access
api-allowed = Allowed { DUR($ms) } ago.
api-outdated = This explanation changed since you agreed, so API access is off. Read it again and allow access to turn it back on.
api-state-off = Off: the API is not called.
api-state-active = Active.
api-state-token-stale = Waiting for Claude Code to renew its sign-in.
api-state-backoff = Paused by the server; next try in { DUR($ms) }.
api-state-offline = Offline; next try in { DUR($ms) }.
api-state-error = The last answer was unusable; next try in { DUR($ms) }.
api-last-success = Last successful request { DUR($ms) } ago.
api-requests = { NUM($requests, 0) } { $requests ->
    [one] request
   *[other] requests
} and { NUM($errors, 0) } { $errors ->
    [one] error
   *[other] errors
} since start.
api-last-error = Last error: { $category }
api-status-unknown = The status is shown once TokenRadar has started.

## Notifications

notify-enabled = Show notifications
notify-five = 5-hour window
notify-weekly = Weekly limits
notify-thresholds-hint = A notification when usage passes each percentage, once per window. Up to { $max }.
notify-threshold-remove = Remove { PCT($pct) }
notify-threshold-new = New threshold in percent
notify-threshold-add = Add
notify-threshold-invalid = Enter a whole number from 1 to 100.
notify-threshold-full = Up to { $max } thresholds.
notify-projection = Warn when a limit will be full before it resets
notify-projection-hint = When the current rate fills it within the hour; at most once per window.
notify-final = Last warning
notify-final-off = Off
notify-final-option = { DUR($ms) } before full
notify-reset = Announce resets
notify-reset-hint = Only after a window that went at least this high, so you do not hear about every quiet window.
notify-reset-always = After every window
notify-reset-option = After { PCT($pct) }
notify-quiet = Quiet hours
notify-quiet-hint = Notifications are kept but not shown. The ones that still matter appear together afterwards.
notify-quiet-from = From
notify-quiet-to = To
notify-test = Send test notification
notify-test-sent = Sent. If nothing appeared, check notifications for TokenRadar in Windows settings.

## Privacy & data

privacy-storage = Storage
privacy-data-dir = Data folder
privacy-portable = Portable: the data stays next to the app.
privacy-db = Database
privacy-logs = Log files
privacy-retention = Keep detailed usage records for
privacy-retention-hint = Older records are deleted. 5-minute summaries are kept, so limits and history stay intact.
privacy-retention-days = { NUM($days, 0) } { $days ->
    [one] day
   *[other] days
}
privacy-network = Network
privacy-network-intro = TokenRadar connects only to these addresses:
privacy-network-api = api.anthropic.com: your limit percentages, only with your permission.
privacy-network-github = github.com: looking for new releases (from a later version).
privacy-network-on = On
privacy-network-off = Off
privacy-network-none = No telemetry, no analytics, nothing else.
privacy-delete = Delete TokenRadar's data
privacy-delete-hint = Deletes TokenRadar's database, logs and settings. Claude Code's files are not touched.
privacy-delete-button = Delete all data…
privacy-delete-title = Delete all TokenRadar data?
privacy-delete-what = Deleted: the usage database, pending statusline readings, log files and your TokenRadar settings.
privacy-delete-kept = Kept: every Claude Code file, and the statusline bridge record, so the bridge can still be removed cleanly. Afterwards TokenRadar reads your sessions again from the start.
privacy-delete-check = I understand that this cannot be undone.
privacy-delete-confirm = Delete everything
privacy-delete-done = TokenRadar's data was deleted.
privacy-delete-partial = Some files could not be deleted; they may be in use. Close other TokenRadar windows and try again.
privacy-diagnostics = Diagnostics bundle
privacy-diagnostics-hint = A zip file to attach to a bug report: versions, counts, sizes and the last 200 log lines. Folder paths are replaced with short codes. Tokens, conversation content, session titles and project names are never included. Nothing is sent; you decide where the file goes.
privacy-diagnostics-button = Create diagnostics bundle…
privacy-diagnostics-done = Saved: { $path }
privacy-diagnostics-failed = The bundle could not be created.
privacy-log-level = Log detail
log-level-error = Errors only
log-level-warn = Warnings
log-level-info = Normal
log-level-debug = Detailed
log-level-trace = Everything

## About

about-version = Version { $version }
about-tagline = Tracks the usage limits of AI coding agents. Local only, no telemetry.
about-updates = Updates
about-update-check = Look for new releases on GitHub
about-update-check-hint = Once a day, only GitHub Releases is contacted. Nothing is installed without your click.
about-update-now = Check now
about-update-install = Install and restart
about-update-checking = Checking for updates…
about-update-current = TokenRadar is up to date.
about-update-available = Version { $version } is available.
about-update-notes = Release notes
about-update-downloading = Downloading the update…
about-update-downloading-pct = Downloading the update: { PCT($pct) }
about-update-check-failed = The check failed. GitHub may be unreachable.
about-update-install-failed = The update could not be installed. Try again, or download it from the releases page.
about-update-portable = This is a portable copy: download the new zip from the releases page.
about-update-checked = Last checked { DUR($ms) } ago.
about-releases = Releases page
about-license = License
about-license-text = TokenRadar is open source under the MIT license.
about-third-party = TokenRadar includes open-source components under their own licenses.
about-third-party-open = Third-party licenses
about-third-party-missing = The list of third-party licenses ships with release builds.
about-repo = Source code
about-issues = Report a problem
about-link-failed = The page could not be opened.

## Terminal font helper (Step 12o)

font-open = Terminal font…
font-title = Terminal font
font-intro = Your terminal draws the statusline with its own font. TokenRadar cannot change that font by itself, but it can install a Nerd Font and select it in Windows Terminal and VS Code.
font-preview-with = The preview uses { $family }, like your terminal once it selects that font.
font-preview-restart = { $family } is installed. The preview draws it after TokenRadar restarts, as programs see a new font only when they start.
font-section-font = Nerd Font
font-status-none = No Nerd Font is installed. Powerline separators and icons show as boxes.
font-status-old = { $family } is installed, but it predates Nerd Fonts 3.4 and lacks the newer progress glyphs.
font-status-full = { $family } is installed.
font-wide-icons = In this variant, icons are wider than one cell, so columns can shift. The "Mono" variant keeps them one cell wide.
font-download = Download and install…
font-consent = TokenRadar downloads this file from GitHub only after you confirm. It checks the file against a pinned SHA-256 and installs four font files for your Windows account only. No administrator rights are needed.
font-download-confirm = Download and install
font-downloading = Downloading
font-progress = { $percent } %
font-installed = { $family } is installed. Select it in your terminal below.
font-install-unsupported = Installing fonts from TokenRadar is not available on this system yet.
font-section-terminal = Terminal
font-terminal-seen = The statusline was last called from { $terminal }.
font-terminal-unknown = No statusline call seen since TokenRadar started. Use Claude Code once, then open this again.
font-terminal-windows-terminal = Windows Terminal
font-terminal-vscode = VS Code
font-terminal-other = another terminal
font-section-settings = Terminal settings
font-no-targets = No Windows Terminal or VS Code settings were found.
font-target-windows-terminal = Windows Terminal
font-target-windows-terminal-preview = Windows Terminal Preview
font-target-windows-terminal-unpackaged = Windows Terminal (portable)
font-target-vscode = VS Code
font-target-vscode-insiders = VS Code Insiders
font-in-use = statusline here
font-current = Font: { $font }
font-current-default = Font: { $font } (terminal default)
font-tier-none = not a Nerd Font
font-tier-old = Nerd Font, before 3.4
font-tier-full = Nerd Font
font-overrides = { $count ->
    [one] One profile sets its own font and keeps it.
   *[other] { $count } profiles set their own font and keep it.
}
font-problem-unreadable = The settings file could not be read.
font-problem-invalid = The settings file is not valid JSON, so TokenRadar leaves it alone.
font-problem-legacy = This file uses the old Windows Terminal format. Choose the font in the terminal's own settings.
font-select = Select font
font-undo = Undo
font-back = Back
font-confirm-select = Select font
font-confirm-undo = Undo change
font-restart-hint = Then close every window of the terminal and open it again: programs see a newly installed font only when they start.
font-selected = The font is selected in { $terminal }.
font-undone = The previous font is back in { $terminal }.
font-already-selected = This font is already selected there.
font-nothing-to-undo = There is nothing to undo there.
font-other-title = Other terminals
font-other-body = In the terminal's settings, choose "{ $family }" as the font. JetBrains IDEs: Settings → Tools → Terminal. Console window: title bar → Properties → Font.
font-error-busy = A download is already running.
font-error-cancelled = The download was cancelled.
font-error-connect = Could not connect to GitHub. Check your connection or proxy.
font-error-timeout = The download timed out.
font-error-status = GitHub answered with status { $code }.
font-error-request = The download failed.
font-error-checksum = The file does not match the expected checksum. Nothing was installed.
font-error-archive = The downloaded archive could not be read. Nothing was installed.
font-error-unsupported = This is not available on this system.
font-error-no-font = Install a Nerd Font first.
font-error-invalid-settings = The settings file is not valid JSON. Nothing was written.
font-error-legacy-settings = This file uses the old Windows Terminal format. Nothing was written.
font-error-changed-meanwhile = The file changed after the diff was shown. Nothing was written. Here is the current change.
font-error-stale-plan = This change was already applied or replaced.
font-error-io = The file could not be read or written.
font-section-restore = Undo everything
font-restore-body = Puts back the terminal fonts TokenRadar changed and removes the font it installed.
font-restore = Restore previous state…
font-restore-intro = This puts everything back as it was before TokenRadar:
font-restore-default = { $font } (terminal default)
font-restore-files = Remove the installed font ({ $count ->
    [one] one file
   *[other] { $count } files
}):
font-restore-note = Each settings file is backed up first. If a terminal still uses the font, close it before you confirm.
font-confirm-restore = Restore
font-restored = Everything is back as it was. The settings backups are next to each file.
font-error-font-in-use = A program still has the font file open, so the font stays installed. The terminal settings are already restored. Close your terminals and try again.

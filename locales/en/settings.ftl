# The settings window (Step 11b): five tabs, the statusline dialog and the delete dialog.

settings-title = Settings
settings-tabs = Settings sections
tab-general = General
tab-sources = Data sources
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
about-update-later = Update checks arrive in a later version. Until then, new releases are on the releases page.
about-releases = Releases page
about-license = License
about-license-text = TokenRadar is open source under the MIT license.
about-third-party = The list of third-party licenses ships with release builds.
about-repo = Source code
about-issues = Report a problem
about-link-failed = The page could not be opened.

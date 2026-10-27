# Security policy

TokenRadar reads local files that sit next to your Claude Code credentials and, with your consent, uses your access token. Security reports are taken seriously.

## Supported versions

Only the latest release receives fixes.

## Reporting a vulnerability

Please **do not open a public issue.** Report privately through GitHub: on the repository's **Security** tab choose **Report a vulnerability** ([direct link](https://github.com/umitkara/tokenradar/security/advisories/new)).

Include what you found, how to reproduce it and the version (`tokenradar --version`). Do not include real tokens or conversation content.

You can expect a first answer within a week. Once a fix is released, the advisory is published with credit to you unless you prefer otherwise.

## Scope

Of particular interest:

- Anything that could expose, persist or log the Claude Code access token, or read the refresh token.
- Conversation content being parsed, stored, shown or sent anywhere.
- Network connections beyond `api.anthropic.com` (with consent) and GitHub Releases.
- Changes to `~/.claude/settings.json` without the user's confirmation, or without a backup.
- Ways to make the updater install a build that was not signed with the project's key.

Not in scope: the SmartScreen warning for the unsigned installer (see the README), and limits of Windows itself.

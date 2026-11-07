// TokenRadar's Claude Code mod (ADR-043).
//
// A companion to the statusline bridge, never a replacement: the bridge stays
// the data source; the mod shows and warns. It reads Claude Code's own figures
// from `session.measure` and asks the TokenRadar CLI, now and then, for what
// only TokenRadar knows (projections, scoped limits, the session share).
//
// It hooks no prompt, compose or tool event, reads no conversation, makes no
// network call and touches no credential.

import { atom, read, update } from 'claude-code'
import type { EngineInterface, Register } from 'claude-code'

import type { AlertState, Measure, Radar } from '../types'
import { t } from './i18n'
import {
  BAND_LINES,
  FETCH_EVERY_MS,
  alertsOf,
  bandLine,
  isBandShown,
  measureFrom,
  mergeRows,
  missingRadar,
  parseStatus,
  report,
  shouldToast,
  stepAlerts,
  toastText,
  topLevel,
} from './model'

const CLI_TIMEOUT_MS = 5_000

const measure = atom({ plugin: 'tokenradar', key: 'measure' } as const, null as Measure | null, {
  shape: 'measure-1',
})
const radar = atom({ plugin: 'tokenradar', key: 'radar' } as const, null as Radar | null, {
  shape: 'radar-1',
})
const alerts = atom(
  { plugin: 'tokenradar', key: 'alerts' } as const,
  { hidden: 0, toasted: 0 } as AlertState,
  { shape: 'alerts-1' },
)

/** TokenRadar's view, at most once per `minAgeMs`; `missing` when the CLI fails. */
async function refresh($: EngineInterface, cli: string, minAgeMs: number): Promise<Radar> {
  const now = await $.clock.now()
  const prev = await read($, radar)
  if (prev !== null && now - prev.at < minAgeMs) return prev
  let next: Radar
  try {
    const sessionId = await $.session.id()
    const run = await $.process.run([cli, 'status', '--json'], { timeoutMs: CLI_TIMEOUT_MS })
    next =
      (run.exitCode === 0 ? parseStatus(run.stdout, sessionId, now) : null) ??
      missingRadar(now, prev?.lang)
  } catch {
    // Not on PATH, not a CLI host, or too slow: Claude Code's figures alone.
    next = missingRadar(now, prev?.lang)
  }
  await update($, radar, () => next)
  return next
}

export const register: Register = (on, options) => {
  const cli =
    typeof options.cli === 'string' && options.cli.trim() !== '' ? options.cli.trim() : 'tokenradar'

  on('session.start', async ($, e, next) => {
    const r = await refresh($, cli, FETCH_EVERY_MS)
    await $.command.register({ name: 'radar', description: t(r.lang).commandDescription })
    try {
      const now = await $.clock.now()
      const usage = await $.session.usage()
      await update($, measure, () => measureFrom(usage, now))
    } catch {
      // No figures yet; the first `session.measure` brings them.
    }
    return next(e)
  })

  on('session.measure', async ($, e, next) => {
    const now = await $.clock.now()
    await update($, measure, () => measureFrom(e, now))
    const r = await refresh($, cli, FETCH_EVERY_MS)

    const found = alertsOf(mergeRows(await read($, measure), r, now))
    const prev = await read($, alerts)
    const step = stepAlerts(prev, topLevel(found))
    if (step.state.hidden !== prev.hidden || step.state.toasted !== prev.toasted) {
      await update($, alerts, () => step.state)
    }
    const first = found[0]
    if (step.toast && first !== undefined && shouldToast(r)) {
      $.ui.toast(toastText(r.lang, first))
    }
    return next(e)
  })

  on('ui.render', { component: 'AbovePrompt' }, async ($, e, next) => {
    if (e.props.hasSurvey) return next(e)
    const now = await $.clock.now()
    const r = await read($, radar)
    const found = alertsOf(mergeRows(await read($, measure), r, now))
    const level = topLevel(found)
    if (!isBandShown(await read($, alerts), level)) return next(e)

    const lang = r?.lang ?? 'en'
    const { Box, Button, Text } = $.ui.resolve(e)
    return (
      <Box flexDirection="row">
        <Box flexDirection="column" flexGrow={1}>
          {found.slice(0, BAND_LINES).map(a => (
            <Text color={a.level === 2 ? 'error' : 'warning'}>{bandLine(lang, a, now)}</Text>
          ))}
        </Box>
        <Button
          key="hide"
          label={t(lang).hide}
          onPress={() => update($, alerts, s => ({ ...s, hidden: level }))}
        />
      </Box>
    )
  })

  on('command.run', { command: 'radar' }, async $ => {
    const r = await refresh($, cli, 0)
    const now = await $.clock.now()
    let m: Measure | null
    try {
      m = measureFrom(await $.session.usage(), now)
      await update($, measure, () => m)
    } catch {
      m = await read($, measure)
    }
    return { text: report(r.lang, mergeRows(m, r, now), r, m, now) }
  })
}

// TokenRadar's Claude Code mod (ADR-043, ADR-046).
//
// A companion to the statusline bridge, never a replacement: the bridge stays
// the data source; the mod shows and warns. It reads Claude Code's own figures
// from `session.measure` and asks the TokenRadar CLI, now and then, for what
// only TokenRadar knows (projections, scoped limits, live sessions, the stored
// history of the window).
//
// It hooks no prompt, compose or tool event, reads no conversation, makes no
// network call and touches no credential. Of Claude Code's settings it reads
// `statusLine.command` alone, to stay quiet where the bridge already shows.

import { atom, read, update } from 'claude-code'
import type { EngineInterface, Register, RenderChildren } from 'claude-code'

import type { AlertState, History, Measure, Radar, ResetState } from '../types'
import { chartCells, chartSvg, seriesFor } from './draw'
import { pct, t } from './i18n'
import {
  FETCH_EVERY_MS,
  HISTORY_EVERY_MS,
  alertsOf,
  bridgeInstalled,
  isBandShown,
  limitName,
  measureFrom,
  mergeRows,
  missingRadar,
  paceRow,
  parseHistory,
  parseStatus,
  report,
  resetNotice,
  rowLevel,
  shouldToast,
  statusText,
  stepAlerts,
  stepResets,
  summary,
  toastLevel,
  toastText,
  topLevel,
  value,
} from './model'
import { bandTree, paneTree, resetTree } from './view'

const CLI_TIMEOUT_MS = 5_000
const PANE = 'tokenradar'
const CHART_ROWS = 6
/** The pane's wished-for height inline: its content, up to what the layout spares. */
const PANE_ROWS = 48

const measure = atom({ plugin: 'tokenradar', key: 'measure' } as const, null as Measure | null, {
  shape: 'measure-1',
})
// radar-2: live sessions added (ADR-046).
const radar = atom({ plugin: 'tokenradar', key: 'radar' } as const, null as Radar | null, {
  shape: 'radar-2',
})
const alerts = atom(
  { plugin: 'tokenradar', key: 'alerts' } as const,
  { hidden: 0, toasted: 0 } as AlertState,
  { shape: 'alerts-1' },
)
const history = atom({ plugin: 'tokenradar', key: 'history' } as const, null as History | null, {
  shape: 'history-1',
})
const pane = atom({ plugin: 'tokenradar', key: 'pane' } as const, false, { shape: 'pane-1' })
const bridge = atom({ plugin: 'tokenradar', key: 'bridge' } as const, null as boolean | null, {
  shape: 'bridge-1',
})
const resets = atom(
  { plugin: 'tokenradar', key: 'resets' } as const,
  { known: {}, notice: null } as ResetState,
  { shape: 'resets-1' },
)

/** The host's offset from UTC in minutes, for clock times. */
const tzOffset = (now: number) => -new Date(now).getTimezoneOffset()

/** Whether Claude Code's statusline is TokenRadar's bridge (ADR-046). */
async function lookBridge($: EngineInterface): Promise<boolean> {
  try {
    const home = (await $.env.get('USERPROFILE')) ?? (await $.env.get('HOME'))
    const dir = (await $.env.get('CLAUDE_CONFIG_DIR')) ?? (home === undefined ? undefined : `${home}/.claude`)
    if (dir === undefined) return false
    return bridgeInstalled(await $.fs.read(`${dir}/settings.json`))
  } catch {
    return false
  }
}

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
  const installed = await lookBridge($)
  if ((await read($, bridge)) !== installed) await update($, bridge, () => installed)
  return next
}

/** The window's stored readings, at most once per `minAgeMs`; none from an older CLI. */
async function refreshHistory($: EngineInterface, cli: string, minAgeMs: number) {
  const now = await $.clock.now()
  const prev = await read($, history)
  if (prev !== null && now - prev.at < minAgeMs) return
  let next: History
  try {
    const run = await $.process.run([cli, 'history', '--json'], { timeoutMs: CLI_TIMEOUT_MS })
    next = (run.exitCode === 0 ? parseHistory(run.stdout, now) : null) ?? { series: [], at: now }
  } catch {
    next = { series: [], at: now }
  }
  await update($, history, () => next)
}

/** The last status entry set, so an unchanged one is not sent again. */
let shownStatus: string | undefined

async function openPane($: EngineInterface, cli: string, lang: 'en' | 'tr'): Promise<boolean> {
  await refreshHistory($, cli, 0)
  const opened = await $.ui.open({ id: PANE, title: t(lang).title, rows: PANE_ROWS })
  await update($, pane, () => true)
  return opened.isPlaced
}

/** Thresholds, toasts, resets and the status entry, from the figures at hand. */
async function settle($: EngineInterface, r: Radar) {
  const now = await $.clock.now()
  const rows = mergeRows(await read($, measure), r, now)

  const found = alertsOf(rows)
  const prev = await read($, alerts)
  const step = stepAlerts(prev, topLevel(found), toastLevel(found))
  if (step.state.hidden !== prev.hidden || step.state.toasted !== prev.toasted) {
    await update($, alerts, () => step.state)
  }
  const first = found[0]
  if (step.toast && first !== undefined && shouldToast(r)) {
    $.ui.toast(toastText(r.lang, first))
  }

  const before = await read($, resets)
  const after = stepResets(before, rows, now)
  if (after !== before) await update($, resets, () => after)

  // The bridge already shows these figures in Claude Code's statusline.
  const status = (await read($, bridge)) === false ? statusText(r.lang, rows) : undefined
  if (status !== shownStatus) {
    shownStatus = status
    $.ui.status(status)
  }
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
    await settle($, r)
    return next(e)
  })

  on('session.measure', async ($, e, next) => {
    const now = await $.clock.now()
    await update($, measure, () => measureFrom(e, now))
    const r = await refresh($, cli, FETCH_EVERY_MS)
    if (await read($, pane)) await refreshHistory($, cli, HISTORY_EVERY_MS)
    await settle($, r)
    return next(e)
  })

  on('ui.close', { id: PANE }, async ($, e, next) => {
    await update($, pane, () => false)
    return next(e)
  })

  on('ui.render', { component: 'AbovePrompt' }, async ($, e, next) => {
    if (e.props.hasSurvey) return next(e)
    const now = await $.clock.now()
    const r = await read($, radar)
    const lang = r?.lang ?? 'en'
    const found = alertsOf(mergeRows(await read($, measure), r, now))
    const level = topLevel(found)
    const els = $.ui.resolve(e)

    if (isBandShown(await read($, alerts), level)) {
      return bandTree(els, lang, found, level, now, e.surface, {
        detail: () => openPane($, cli, lang),
        hide: () => update($, alerts, s => ({ ...s, hidden: level })),
      })
    }
    const notice = resetNotice(await read($, resets), now)
    if (notice !== null) {
      return resetTree(els, lang, notice.id, e.surface, () =>
        update($, resets, s => ({ ...s, notice: null })),
      )
    }
    return next(e)
  })

  on('ui.render', { component: 'Pane', requestId: PANE }, async ($, e) => {
    const now = await $.clock.now()
    const r = await read($, radar)
    const m = await read($, measure)
    const hist = await read($, history)
    const lang = r?.lang ?? 'en'
    const rows = mergeRows(m, r, now)
    const width = Math.max(20, e.props.bodyColumns)
    const tzOffsetMin = tzOffset(now)

    // The chart of the time strip's window, when TokenRadar stored readings of it.
    const row = paceRow(rows)
    const series = row === undefined ? undefined : seriesFor(hist?.series ?? [], row)
    let chart: RenderChildren | null = null
    if (row !== undefined && series !== undefined) {
      const level = rowLevel(row)
      if (e.surface === 'terminal') {
        const { Raster } = $.ui.resolve(e)
        // As wide as the strip above it, so days line up.
        // Raster takes at most 512 columns.
        const columns = Math.max(14, Math.floor(Math.min(width, 511) / 7) * 7)
        chart = (
          <Raster
            key="chart"
            columns={columns}
            rows={CHART_ROWS}
            cells={chartCells(series, row, level, now, columns, CHART_ROWS)}
          />
        )
      } else {
        const { Svg } = $.ui.resolve(e)
        const d = t(lang)
        chart = (
          <Svg
            source={chartSvg(lang, series, row, level, now, tzOffsetMin)}
            alt={d.chartAlt(limitName(lang, row.id), value(lang, row), pct(lang, row.elapsed ?? 0))}
          />
        )
      }
    }

    return paneTree($.ui.resolve(e), {
      lang,
      rows,
      radar: r,
      measure: m,
      now,
      tzOffsetMin,
      width,
      surface: e.surface,
      chart,
    })
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
    const rows = mergeRows(m, r, now)
    // The pane shows the rest; the answer, which the model reads too, stays short.
    if (await openPane($, cli, r.lang)) return { text: summary(r.lang, rows) }
    return { text: report(r.lang, rows, r, m, now, tzOffset(now)) }
  })
}

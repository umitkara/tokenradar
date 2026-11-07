// Pure rules of the mod: reading TokenRadar's answer, merging it with Claude
// Code's own figures, thresholds, and the text the band, toast and /radar show.
// Nothing here touches `$`, so the tests drive it directly.

import type { SessionUsage } from 'claude-code'

import type { AlertState, Level, Measure, Radar, RadarLimit, Window } from '../types'
import { dur, num, pct, t, type Lang } from './i18n'

/** The tray's and the statusline's thresholds (ADR-042). */
export const WARN_AT = 75
export const CRIT_AT = 90
/** At most one CLI run per this many milliseconds (ADR-043). */
export const FETCH_EVERY_MS = 30_000
/** Lines the band shows at most. */
export const BAND_LINES = 3
/** Points of use ahead of the time line that the band warns about (ADR-044). */
export const PACE_AHEAD = 10

const HOUR_MS = 3_600_000
const DAY_MS = 24 * HOUR_MS

/** How long a window is, by row id; `null` when unknown. */
export function windowLength(id: string): number | null {
  if (id === 'five_hour') return 5 * HOUR_MS
  if (id === 'seven_day' || id.startsWith('scoped:')) return 7 * DAY_MS
  return null
}

/** The share of the window's time that has passed, %: a clock reading, not an estimate. */
export function elapsedOf(id: string, resetsAt: number | undefined, now: number): number | undefined {
  const length = windowLength(id)
  if (length === null || resetsAt === undefined) return undefined
  const left = resetsAt - now
  return left > 0 && left <= length ? (1 - left / length) * 100 : undefined
}

export type Row = {
  id: string
  percent: number
  resetsAt?: number
  /** `claude_code` for Claude Code's own figure, otherwise TokenRadar's source. */
  source: string
  estimated: boolean
  stale: boolean
  observedAt?: number
  projected: boolean
  willHit: boolean
  etaAt?: number
  /** How much of the window's time has passed, %. */
  elapsed?: number
}

/** `pace`: use runs ahead of the time line; shown on the band, never toasted (ADR-044). */
export type Alert = { row: Row; level: Level; reason: 'used' | 'projection' | 'pace' }

// --- Input ---------------------------------------------------------------

const ccId = (kind: string) =>
  kind === 'five_hour' || kind === 'seven_day' ? kind : `other:${kind}`

function radarId(kind: string, scope: unknown): string {
  if (kind === 'session') return 'five_hour'
  if (kind === 'weekly_all') return 'seven_day'
  if (kind === 'weekly_scoped' && typeof scope === 'string') return `scoped:${scope}`
  return `other:${kind}`
}

export function measureFrom(
  u: Pick<SessionUsage, 'context' | 'rateLimits' | 'cost'>,
  now: number,
): Measure {
  const windows: Window[] = u.rateLimits.map(w => {
    const resetsAt = w.resetsAt === undefined ? NaN : Date.parse(w.resetsAt)
    return Number.isFinite(resetsAt)
      ? { kind: w.kind, percentUsed: w.percentUsed, resetsAt }
      : { kind: w.kind, percentUsed: w.percentUsed }
  })
  const m: Measure = { windows, contextWindow: u.context.window, at: now }
  if (u.context.percent !== undefined) m.contextPercent = u.context.percent
  if (u.cost !== undefined) m.costUsd = u.cost.usd
  return m
}

const isObject = (v: unknown): v is Record<string, unknown> =>
  typeof v === 'object' && v !== null && !Array.isArray(v)
const optNumber = (v: unknown) => (typeof v === 'number' && Number.isFinite(v) ? v : undefined)

/**
 * `tokenradar status --json` (schema 1) reduced to what the mod needs; `null`
 * when it is not that. Unknown fields are ignored, so an older or newer CLI works.
 */
export function parseStatus(stdout: string, sessionId: string, now: number): Radar | null {
  let v: unknown
  try {
    v = JSON.parse(stdout)
  } catch {
    return null
  }
  if (!isObject(v) || v.schema !== 1 || !Array.isArray(v.limits)) return null
  const origin = v.origin === 'app' || v.origin === 'database' ? v.origin : null
  if (origin === null) return null

  const limits: RadarLimit[] = []
  for (const l of v.limits) {
    if (!isObject(l) || typeof l.kind !== 'string') continue
    const percent = optNumber(l.percent)
    const observedAt = optNumber(l.observed_at)
    if (percent === undefined || observedAt === undefined) continue
    const limit: RadarLimit = {
      id: radarId(l.kind, l.scope),
      percent,
      source: typeof l.source === 'string' ? l.source : 'estimate',
      estimated: l.estimated === true,
      stale: l.freshness === 'stale',
      observedAt,
      projected: typeof l.projection_confidence === 'string',
      willHit: l.will_hit_before_reset === true,
    }
    const resetsAt = optNumber(l.resets_at)
    if (resetsAt !== undefined) limit.resetsAt = resetsAt
    const etaAt = optNumber(l.eta_100_at)
    if (etaAt !== undefined) limit.etaAt = etaAt
    limits.push(limit)
  }

  const absent = Array.isArray(v.absent)
    ? v.absent.filter(isObject).flatMap(a => (typeof a.kind === 'string' ? [radarId(a.kind, a.scope)] : []))
    : []

  const radar: Radar = {
    origin,
    lang: v.locale === 'tr' ? 'tr' : 'en',
    limits,
    absent,
    at: now,
  }
  const own = Array.isArray(v.live_sessions)
    ? v.live_sessions.find(s => isObject(s) && s.session_uuid === sessionId)
    : undefined
  const share = isObject(own) ? optNumber(own.share_percent) : undefined
  if (share !== undefined) radar.share = share
  return radar
}

export const missingRadar = (now: number, lang: Lang = 'en'): Radar => ({
  origin: 'missing',
  lang,
  limits: [],
  absent: [],
  at: now,
})

// --- Merge ---------------------------------------------------------------

const ORDER = (id: string) =>
  id === 'five_hour' ? 0 : id === 'seven_day' ? 1 : id.startsWith('scoped:') ? 2 : 3

/**
 * Claude Code's own figure wins for a window it reports (it is the statusline's
 * source and the freshest); TokenRadar adds its projection to it, and the windows
 * Claude Code does not report (scoped limits). A window past its reset shows
 * `≈ 0 %`, as the statusline does (ADR-042).
 */
export function mergeRows(m: Measure | null, r: Radar | null, now: number): Row[] {
  const fromRadar = new Map((r?.limits ?? []).map(l => [l.id, l]))
  const absent = new Set(r?.absent ?? [])
  const out: Row[] = []

  for (const w of m?.windows ?? []) {
    const id = ccId(w.kind)
    const tr = fromRadar.get(id)
    fromRadar.delete(id)
    const row: Row = {
      id,
      percent: w.percentUsed,
      source: 'claude_code',
      estimated: false,
      stale: false,
      projected: tr?.projected ?? false,
      willHit: tr?.willHit ?? false,
    }
    const resetsAt = w.resetsAt ?? tr?.resetsAt
    if (resetsAt !== undefined) row.resetsAt = resetsAt
    if (tr?.etaAt !== undefined) row.etaAt = tr.etaAt
    out.push(row)
  }
  for (const l of fromRadar.values()) {
    if (absent.has(l.id)) continue
    const row: Row = {
      id: l.id,
      percent: l.percent,
      source: l.source,
      estimated: l.estimated,
      stale: l.stale,
      observedAt: l.observedAt,
      projected: l.projected,
      willHit: l.willHit,
    }
    if (l.resetsAt !== undefined) row.resetsAt = l.resetsAt
    if (l.etaAt !== undefined) row.etaAt = l.etaAt
    out.push(row)
  }

  return out
    .map(row => {
      if (row.resetsAt === undefined || row.resetsAt > now) {
        const elapsed = elapsedOf(row.id, row.resetsAt, now)
        return elapsed === undefined ? row : { ...row, elapsed }
      }
      const { resetsAt: _, etaAt: __, ...rest } = row
      return { ...rest, percent: 0, estimated: true, willHit: false, projected: false }
    })
    .sort((a, b) => ORDER(a.id) - ORDER(b.id))
}

// --- Thresholds ----------------------------------------------------------

export function rowLevel(row: Row): Level {
  if (row.percent >= CRIT_AT) return 2
  if (row.percent >= WARN_AT || row.willHit) return 1
  return 0
}

/** Points of use ahead of the time line; `undefined` without a time reading. */
export const aheadBy = (row: Row) =>
  row.elapsed === undefined ? undefined : row.percent - row.elapsed

/** Rows past a threshold or ahead of time, the most urgent first. */
export function alertsOf(rows: readonly Row[]): Alert[] {
  return rows
    .flatMap((row): Alert[] => {
      const level = rowLevel(row)
      if (level === 0) {
        // An estimate is not measured against the clock: it would warn on a guess.
        const ahead = aheadBy(row)
        return ahead !== undefined && ahead >= PACE_AHEAD && !row.estimated
          ? [{ row, level: 1, reason: 'pace' }]
          : []
      }
      const reason: Alert['reason'] =
        row.percent >= WARN_AT || row.etaAt === undefined ? 'used' : 'projection'
      return [{ row, level, reason }]
    })
    .sort((a, b) => b.level - a.level || b.row.percent - a.row.percent)
}

export const topLevel = (alerts: readonly Alert[]): Level => alerts[0]?.level ?? 0

/** The level that may raise a toast: a pace alert only shows on the band. */
export const toastLevel = (alerts: readonly Alert[]): Level =>
  alerts.find(a => a.reason !== 'pace')?.level ?? 0

/**
 * A toast once per crossing: when the toast level rises past the last one toasted.
 * With nothing left on the band (a reset), both the toast and the band's "hide"
 * start over; a pace alert keeps the band's "hide" as it is.
 */
export function stepAlerts(
  prev: AlertState,
  bandLevel: Level,
  toastable: Level = bandLevel,
): { state: AlertState; toast: boolean } {
  if (bandLevel === 0) return { state: { hidden: 0, toasted: 0 }, toast: false }
  const toast = toastable > prev.toasted
  return { state: { hidden: prev.hidden, toasted: toast ? toastable : prev.toasted }, toast }
}

export const isBandShown = (state: AlertState, level: Level) => level > 0 && level > state.hidden

/** The desktop app sends its own notification while it runs (ADR-043). */
export const shouldToast = (r: Radar | null) => r?.origin !== 'app'

// --- Text ----------------------------------------------------------------

export function limitName(lang: Lang, id: string): string {
  const d = t(lang)
  if (id === 'five_hour') return d.limitFiveHour
  if (id === 'seven_day') return d.limitWeekly
  if (id.startsWith('scoped:')) return d.limitWeeklyScoped(id.slice(7).replace(/^model:/, ''))
  const kind = id.replace(/^other:/, '')
  return kind === 'spend_limit' ? d.limitSpend : kind
}

export function sourceName(lang: Lang, source: string): string {
  const d = t(lang)
  switch (source) {
    case 'claude_code':
      return d.sourceClaudeCode
    case 'statusline':
      return d.sourceStatusline
    case 'api':
      return d.sourceApi
    case 'estimate':
      return d.sourceEstimate
    case 'codex_log':
      return d.sourceCodexLog
    default:
      return source
  }
}

const value = (lang: Lang, row: Row) => `${row.estimated ? '≈' : ''}${pct(lang, row.percent)}`

export function bandLine(lang: Lang, a: Alert, now: number): string {
  const d = t(lang)
  const name = limitName(lang, a.row.id)
  const ahead = aheadBy(a.row)
  if (a.reason === 'pace' && ahead !== undefined && a.row.elapsed !== undefined) {
    const text = d.bandPace(name, num(lang, ahead, 0), pct(lang, a.row.percent), pct(lang, a.row.elapsed))
    return `${text} · ${sourceName(lang, a.row.source)}`
  }
  if (a.reason === 'projection' && a.row.etaAt !== undefined) {
    return `${d.bandProjection(name, dur(lang, a.row.etaAt - now))} · TokenRadar`
  }
  const resets = a.row.resetsAt === undefined ? null : dur(lang, a.row.resetsAt - now)
  return `${d.bandUsed(name, value(lang, a.row), resets)} · ${sourceName(lang, a.row.source)}`
}

export function toastText(lang: Lang, a: Alert): string {
  const d = t(lang)
  const name = limitName(lang, a.row.id)
  return a.reason === 'projection'
    ? d.toastProjection(name)
    : d.toastUsed(name, value(lang, a.row))
}

/** The `/radar` report: limits with sources, absent windows, session figures, data origin. */
export function report(
  lang: Lang,
  rows: readonly Row[],
  r: Radar | null,
  m: Measure | null,
  now: number,
  tzOffsetMin = 0,
): string {
  const d = t(lang)
  const lines: string[] = [d.title, '']

  if (rows.length === 0) {
    lines.push(d.noLimits)
  } else {
    const table = [
      [d.colLimit, d.colUsed, d.colResets, d.colSource, d.colProjection],
      ...rows.map(row => [
        limitName(lang, row.id),
        value(lang, row),
        row.resetsAt === undefined ? '—' : dur(lang, row.resetsAt - now),
        row.stale && row.observedAt !== undefined
          ? `${sourceName(lang, row.source)} (${d.updatedAgo(dur(lang, now - row.observedAt))})`
          : sourceName(lang, row.source),
        row.willHit && row.etaAt !== undefined
          ? `≈${d.fullIn(dur(lang, row.etaAt - now))}`
          : row.projected
            ? d.resetFirst
            : '—',
      ]),
    ]
    const widths = table[0]!.map((_, c) => Math.max(...table.map(cells => cells[c]!.length)))
    for (const cells of table) {
      const text = cells
        .map((cell, c) => (c === 1 ? cell.padStart(widths[c]!) : cell.padEnd(widths[c]!)))
        .join('  ')
      lines.push(text.trimEnd())
    }
  }
  for (const id of r?.absent ?? []) lines.push(d.absent(limitName(lang, id)))
  lines.push(...paceLines(lang, rows, now, tzOffsetMin))

  const session: string[] = []
  if (r?.share !== undefined) session.push(d.share(pct(lang, r.share)))
  if (m?.contextPercent !== undefined) {
    session.push(d.context(pct(lang, m.contextPercent), num(lang, m.contextWindow, 0)))
  }
  if (m?.costUsd !== undefined) session.push(d.cost(num(lang, m.costUsd, 2)))
  if (session.length > 0) lines.push('', ...session)

  lines.push('')
  lines.push(
    r === null || r.origin === 'missing'
      ? d.dataNone
      : r.origin === 'app'
        ? d.dataApp
        : d.dataDatabase,
  )
  const anyEstimate =
    rows.some(row => row.estimated || (row.willHit && row.etaAt !== undefined)) ||
    r?.share !== undefined
  if (anyEstimate) lines.push(d.estimateNote)
  return lines.join('\n')
}

// --- Pace (ADR-044) ------------------------------------------------------

/** The weekly window's row with a time reading, else the 5-hour window's. */
const paceRow = (rows: readonly Row[]) =>
  rows.find(r => r.id === 'seven_day' && r.elapsed !== undefined) ??
  rows.find(r => r.id === 'five_hour' && r.elapsed !== undefined)

/**
 * `/radar`'s pace: how much of the window's time has passed against how much was
 * used, then, for the weekly window, where the time line stands at the end of each
 * coming day of the window, so use can be read against it.
 */
export function paceLines(
  lang: Lang,
  rows: readonly Row[],
  now: number,
  tzOffsetMin: number,
): string[] {
  const row = paceRow(rows)
  if (row?.elapsed === undefined || row.resetsAt === undefined) return []
  const length = windowLength(row.id)
  if (length === null) return []
  const d = t(lang)
  const ahead = row.percent - row.elapsed
  const out = [
    '',
    d.paceLine(
      limitName(lang, row.id),
      pct(lang, row.elapsed),
      value(lang, row),
      num(lang, Math.abs(ahead), 0),
      dur(lang, (Math.abs(ahead) / 100) * length),
      ahead > 0,
    ),
  ]
  if (row.id === 'seven_day') {
    const start = row.resetsAt - length
    const marks: string[] = []
    for (let day = 1; day <= 7; day++) {
      const at = start + day * DAY_MS
      if (at <= now) continue
      const when = clock(lang, at, tzOffsetMin)
      marks.push(day === 7 ? `${when} ${d.resets}` : `${when} ${pct(lang, (day / 7) * 100)}`)
    }
    out.push(`${d.timeLine}: ${marks.join(' · ')}`)
  }
  return out
}

/** `Fri 11:00` / `Cum 11:00`, at the given offset from UTC in minutes. */
export function clock(lang: Lang, at: number, tzOffsetMin: number): string {
  const local = new Date(at + tzOffsetMin * 60_000)
  const day = t(lang).weekdays[local.getUTCDay()] ?? ''
  const hh = String(local.getUTCHours()).padStart(2, '0')
  const mm = String(local.getUTCMinutes()).padStart(2, '0')
  return `${day} ${hh}:${mm}`
}

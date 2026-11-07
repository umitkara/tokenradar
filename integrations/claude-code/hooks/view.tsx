// The mod's trees (ADR-046): the pane `/radar` opens and the one-line band. Every
// surface draws the same Box/Text tree; the chart under the time strip is the one
// part that differs, an Svg on the desktop, VS Code and mobile, a Raster in the
// terminal, handed in by the caller that knows the surface.

import type { ElementTable, RenderChildren } from 'claude-code'

import type { Level, Measure, Radar } from '../types'
import { bar, levelTone, strip, toneColor, type Seg, type Tone } from './draw'
import { dur, num, pct, t, type Lang } from './i18n'
import {
  type Alert,
  type Row,
  bandLine,
  limitName,
  paceRow,
  rowLevel,
  sourceName,
  value,
} from './model'

type Els = ElementTable

/** Colored runs as one line of text. */
function Runs(els: Els, segs: readonly Seg[], surface: string) {
  const { Text } = els
  return segs.map(s => <Text color={toneColor(s.tone, surface)}>{s.text}</Text>)
}

/** The source as a colored badge; an old value's badge is dim (design-language: honest numbers). */
function badge(els: Els, lang: Lang, row: Row, surface: string) {
  const { Text } = els
  const color = row.stale
    ? 'inactive'
    : row.source === 'claude_code'
      ? 'claude'
      : row.source === 'statusline'
        ? toneColor('ok', surface)
        : row.source === 'api'
          ? 'suggestion'
          : 'inactive'
  return (
    <Text backgroundColor={color} color="inverseText">
      {` ${row.estimated ? '≈ ' : ''}${sourceName(lang, row.source)} `}
    </Text>
  )
}

/** One limit: name and source, the bar with the time mark, reset, projection, freshness. */
function card(els: Els, lang: Lang, row: Row, width: number, now: number, surface: string) {
  const { Box, Text } = els
  const d = t(lang)
  const level = rowLevel(row)
  const tone = levelTone(level)
  const dim = row.stale || row.estimated
  const shown = value(lang, row)
  // The time mark stays on the bar; the elapsed share is written once, on the strip.
  const inner = Math.max(10, width - 4)
  const barWidth = Math.max(4, inner - shown.length - 1)
  const details: string[] = []
  if (row.resetsAt !== undefined) details.push(d.resetsIn(dur(lang, row.resetsAt - now)))
  if (row.willHit && row.etaAt !== undefined) {
    details.push(`${d.colProjection}: ≈${d.fullIn(dur(lang, row.etaAt - now))}`)
  } else if (row.projected) {
    details.push(`${d.colProjection}: ${d.resetFirst}`)
  }
  if (row.stale && row.observedAt !== undefined) {
    details.push(d.updatedAgo(dur(lang, now - row.observedAt)))
  }
  return (
    <Box
      key={`card-${row.id}`}
      flexDirection="column"
      borderStyle="round"
      borderColor={level === 0 ? 'subtle' : toneColor(tone, surface)}
      borderDimColor={dim}
      paddingX={1}
    >
      <Box flexDirection="row" justifyContent="space-between">
        <Text bold dimColor={dim}>
          {limitName(lang, row.id)}
        </Text>
        {badge(els, lang, row, surface)}
      </Box>
      <Text wrap="truncate-end">
        {Runs(els, bar(row.percent, row.elapsed, barWidth, tone, row.estimated), surface)}
        <Text bold dimColor={dim}>{` ${shown}`}</Text>
      </Text>
      {details.length > 0 && (
        <Text dimColor wrap="truncate-end">
          {details.join(' · ')}
        </Text>
      )}
    </Box>
  )
}

export type PaneInput = {
  lang: Lang
  rows: readonly Row[]
  radar: Radar | null
  measure: Measure | null
  now: number
  tzOffsetMin: number
  width: number
  surface: string
  /** The chart under the time strip, drawn for this surface; `null` without history. */
  chart: RenderChildren | null
}

/** The pane: limit cards, the weekly time strip and chart, live sessions, context, data origin. */
export function paneTree(els: Els, p: PaneInput) {
  const { Box, Text } = els
  const { lang, rows, radar: r, measure: m, now, width, surface } = p
  const d = t(lang)
  const sections: RenderChildren[] = []

  if (rows.length === 0) sections.push(<Text dimColor>{d.noLimits}</Text>)
  for (const row of rows) sections.push(card(els, lang, row, width, now, surface))
  for (const id of r?.absent ?? []) {
    sections.push(<Text dimColor>{d.absent(limitName(lang, id))}</Text>)
  }

  const weekly = paceRow(rows)
  const s = weekly === undefined ? null : strip(lang, weekly, levelTone(rowLevel(weekly)), width, p.tzOffsetMin)
  if (weekly !== undefined && s !== null) {
    sections.push(
      <Box key="strip" flexDirection="column" marginTop={1}>
        <Text bold>{d.timeStrip(limitName(lang, weekly.id), pct(lang, weekly.elapsed ?? 0))}</Text>
        <Text dimColor>{s.labels}</Text>
        <Text>{Runs(els, s.cells, surface)}</Text>
        <Text wrap="truncate-end">
          <Text dimColor>{s.before}</Text>
          <Text bold>{s.now}</Text>
          <Text dimColor>{s.after}</Text>
        </Text>
      </Box>,
    )
  }
  if (p.chart !== null) sections.push(p.chart)
  // The strip's legend under the chart, so it reads as the key to both.
  if (s !== null) {
    const key = p.chart === null ? '' : surface === 'terminal' ? d.chartKeyCells : d.chartKeySvg
    sections.push(<Text key="legend" dimColor>{d.stripLegend(s.reset) + key}</Text>)
  }

  const live = sessionsSection(els, lang, r, width, surface)
  if (live !== null) sections.push(live)

  const ctx: RenderChildren[] = []
  if (m?.contextPercent !== undefined) {
    const label = `${d.contextBar} `
    const tail = ` ${pct(lang, m.contextPercent)} · ${d.tokens(num(lang, m.contextWindow, 0))}`
    const w = Math.max(4, Math.min(30, width - label.length - tail.length))
    ctx.push(
      <Text wrap="truncate-end">
        <Text bold>{label}</Text>
        {Runs(els, bar(m.contextPercent, undefined, w, 'ok'), surface)}
        <Text>{tail}</Text>
      </Text>,
    )
  }
  if (m?.costUsd !== undefined) ctx.push(<Text dimColor>{d.cost(num(lang, m.costUsd, 2))}</Text>)
  if (ctx.length > 0) {
    sections.push(
      <Box key="context" flexDirection="column" marginTop={1}>
        {ctx}
      </Box>,
    )
  }

  const foot: string[] = [
    r === null || r.origin === 'missing' ? d.dataNone : r.origin === 'app' ? d.dataApp : d.dataDatabase,
  ]
  const anyEstimate =
    rows.some(row => row.estimated || (row.willHit && row.etaAt !== undefined)) ||
    (r?.sessions ?? []).some(x => x.share !== undefined)
  if (anyEstimate) foot.push(d.estimateNote)
  sections.push(
    <Box key="foot" flexDirection="column" marginTop={1}>
      {foot.map(line => (
        <Text dimColor>{line}</Text>
      ))}
    </Box>,
  )

  return <Box flexDirection="column">{sections}</Box>
}

const SESSIONS_SHOWN = 5

/**
 * Live sessions: a bar each, of its ≈ share of the 5-hour window when TokenRadar
 * has one (an estimate, so `▒`), else of its part of the live sessions' cost.
 * This session first, the rest by cost.
 */
function sessionsSection(els: Els, lang: Lang, r: Radar | null, width: number, surface: string) {
  const { Box, Text } = els
  const list = r?.sessions ?? []
  if (list.length === 0) return null
  const d = t(lang)
  const sorted = [...list].sort((a, b) => Number(b.own) - Number(a.own) || b.costUsd - a.costUsd)
  const byShare = list.some(x => x.share !== undefined)
  const total = list.reduce((sum, x) => sum + x.costUsd, 0)
  // A cost bar compares sessions: with one costed session it would only ever be full.
  const hasBars = byShare || list.filter(x => x.costUsd > 0).length >= 2
  const names = sorted.map(x => (x.own ? `● ${d.thisSession}` : `  ${x.label}`))
  const nameW = Math.min(24, Math.max(...names.map(n => n.length)))
  const lines = sorted.slice(0, SESSIONS_SHOWN).map((x, i) => {
    const name = names[i]!.slice(0, nameW).padEnd(nameW)
    const figures = [
      x.share === undefined ? '' : `≈${pct(lang, x.share)}`,
      x.costUsd > 0 ? `API $${num(lang, x.costUsd, 2)}` : '',
    ]
      .filter(Boolean)
      .join('  ')
    const barW = Math.max(4, Math.min(16, width - nameW - figures.length - 3))
    const part = byShare ? (x.share ?? 0) : total > 0 ? (x.costUsd / total) * 100 : 0
    const tone: Tone = 'ok'
    return (
      <Text wrap="truncate-end">
        <Text bold={x.own}>{name}</Text>
        {hasBars ? <Text> </Text> : null}
        {hasBars && part > 0 ? Runs(els, bar(part, undefined, barW, tone, byShare), surface) : null}
        {hasBars && part <= 0 ? <Text>{' '.repeat(barW)}</Text> : null}
        <Text dimColor>{figures === '' ? '' : ` ${figures}`}</Text>
      </Text>
    )
  })
  const more = sorted.length - SESSIONS_SHOWN
  return (
    <Box key="sessions" flexDirection="column" marginTop={1}>
      <Text bold>{d.liveSessions}</Text>
      {lines}
      {more > 0 && <Text dimColor>{d.more(more)}</Text>}
      {!hasBars && r?.origin !== 'app' && <Text dimColor>{d.costLater}</Text>}
    </Box>
  )
}

export type BandActions = { detail: () => unknown; hide: () => unknown }

/** The band: one bordered line in the alert's color, a mini bar with the time mark, two buttons. */
export function bandTree(
  els: Els,
  lang: Lang,
  alerts: readonly Alert[],
  level: Level,
  now: number,
  surface: string,
  act: BandActions,
) {
  const { Box, Button, Text } = els
  const d = t(lang)
  const first = alerts[0]!
  const tone = levelTone(level)
  const more = alerts.length - 1
  return (
    <Box
      flexDirection="row"
      borderStyle="round"
      borderColor={toneColor(tone, surface)}
      paddingX={1}
      gap={1}
    >
      <Box flexGrow={1} flexShrink={1}>
        <Text wrap="truncate-end">
          {Runs(els, bar(first.row.percent, first.row.elapsed, 10, levelTone(first.level), first.row.estimated), surface)}
          <Text>{` ${bandLine(lang, first, now)}`}</Text>
          <Text dimColor>{more > 0 ? ` ${d.more(more)}` : ''}</Text>
        </Text>
      </Box>
      <Button key="detail" label={d.detail} onPress={act.detail} />
      <Button key="hide" label={d.hide} onPress={act.hide} />
    </Box>
  )
}

/** The one-off line for a window that just reset. */
export function resetTree(els: Els, lang: Lang, id: string, surface: string, ok: () => unknown) {
  const { Box, Button, Text } = els
  const d = t(lang)
  return (
    <Box flexDirection="row" borderStyle="round" borderColor={toneColor('ok', surface)} paddingX={1} gap={1}>
      <Box flexGrow={1} flexShrink={1}>
        <Text wrap="truncate-end">{`✓ ${d.resetDone(limitName(lang, id))}`}</Text>
      </Box>
      <Button key="reset-ok" label={d.ok} onPress={ok} />
    </Box>
  )
}

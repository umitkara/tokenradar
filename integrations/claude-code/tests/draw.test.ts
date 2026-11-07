// The visual mod's pure parts (ADR-046): drawing helpers and the new rules of
// model.ts (history, bridge, resets, the status entry, live sessions). Every
// value here is synthetic.

import { expect, test } from 'claude-code/testing'

import { bar, base64, chartCells, chartSvg, seriesFor, strip } from '../hooks/draw'
import {
  bridgeInstalled,
  parseHistory,
  parseStatus,
  resetNotice,
  statusText,
  stepResets,
  summary,
  type Row,
} from '../hooks/model'
import type { ResetState, Series } from '../types'

const NOW = Date.parse('2026-11-07T12:00:00Z')
const H = 3_600_000
const DAY = 24 * H

const weekly = (extra: Partial<Row> = {}): Row => ({
  id: 'seven_day',
  percent: 30,
  resetsAt: NOW + 50 * H,
  source: 'claude_code',
  estimated: false,
  stale: false,
  projected: false,
  willHit: false,
  elapsed: (1 - 50 / 168) * 100,
  ...extra,
})

const text = (segs: { text: string }[]) => segs.map(s => s.text).join('')

test('a bar fills the used share and marks the elapsed time in its cell', () => {
  expect(text(bar(50, 25, 8, 'ok'))).toBe('██│█░░░░')
  expect(text(bar(50, undefined, 8, 'ok', true))).toBe('▒▒▒▒░░░░')
  expect(text(bar(140, 100, 4, 'crit'))).toBe('███│')
  // Runs of one tone come merged: fill, mark, fill, track.
  expect(bar(50, 25, 8, 'warn').map(s => s.tone)).toEqual(['warn', 'mark', 'warn', 'track'])
})

test('the weekly strip is calendar days cut at local midnight, the window at the reset hour', () => {
  // Resets Wed 11:00 at +03:00: the window runs Wed 11:00 to Wed 11:00, so the
  // first day has 13 hours, the last 11, and 8 cells a day at 56 cells.
  const resetsAt = Date.parse('2026-11-11T08:00:00Z')
  const row = weekly({ resetsAt, elapsed: (1 - (resetsAt - NOW) / (7 * DAY)) * 100 })
  const s = strip('tr', row, 'ok', 56, 180)
  expect(s).not.toBeNull()
  expect(s!.labels).toBe('Çar Per     Cum     Cmt     Paz     Pzt     Sal     Çar')
  expect(text(s!.cells)).toBe(`${'█'.repeat(17)}░░░┊${'░░░░░░░┊'.repeat(4)}░░░`)
  expect(s!.pointer).toBe(`${' '.repeat(25)}▲ şimdi`)
  // The 24-hour periods from the reset hour, between the midnights.
  expect(s!.periods).toBe(`${' '.repeat(8)}╵%14    ╵%29    ╵%43    ╵%57    ╵%71    ╵%86`)
  expect(s!.reset).toBe('Çar 11:00')
  // With room, the first day says the reset hour.
  expect(strip('tr', row, 'ok', 140, 180)!.labels.startsWith('Çar 11:00  Per')).toBe(true)
  // The pointer turns left near the end; too narrow a strip is left out.
  expect(strip('en', weekly({ elapsed: 99 }), 'ok', 35, 0)!.pointer).toBe(`${' '.repeat(30)}now ▲`)
  expect(strip('en', weekly(), 'ok', 34, 0)).toBeNull()
  expect(strip('en', weekly({ id: 'five_hour' }), 'ok', 56, 0)).toBeNull()
})

const series: Series = {
  id: 'seven_day',
  start: NOW + 50 * H - 7 * DAY,
  resetsAt: NOW + 50 * H,
  points: [
    [NOW + 50 * H - 6 * DAY, 10],
    [NOW - H, 28],
  ],
}

test('the Raster chart packs columns × rows cells of three words each', () => {
  const cells = chartCells(series, weekly(), 0, NOW, 14, 6, 180)
  expect(cells.length).toBe(Math.ceil((14 * 6 * 12) / 3) * 4)
  expect(base64(Uint8Array.of(1, 2, 3, 4))).toBe('AQIDBA==')
  expect(base64(Uint8Array.of(0xff, 0xfe))).toBe('//4=')
})

test('the SVG chart has the curve, the time line, the thresholds and day labels', () => {
  const svg = chartSvg('tr', series, weekly(), 0, NOW, 180)
  expect(svg.startsWith('<svg')).toBe(true)
  expect(svg).toContain('class="t"')
  expect(svg).toContain('class="ok"')
  expect(svg).toContain('>Pzt<')
  expect(svg).toContain('prefers-color-scheme:dark')
  expect(svg.length).toBeLessThan(131_072)
  expect(chartSvg('en', series, weekly({ estimated: true }), 1, NOW, 0)).toContain('stroke-dasharray="6 4"')
  // A series of another window is not drawn for the row.
  expect(seriesFor([series], weekly())).toBe(series)
  expect(seriesFor([series], weekly({ resetsAt: NOW + 3 * DAY }))).toBeUndefined()
})

test('the history answer is read, an unknown one is not', () => {
  const h = parseHistory(
    JSON.stringify({
      schema: 1,
      generated_at: NOW,
      series: [
        {
          kind: 'weekly_scoped',
          scope: 'model:Fable',
          window_start: 1,
          resets_at: 2,
          points: [{ at: 1, percent: 3, source: 'api' }, { at: 'x' }],
        },
      ],
    }),
    NOW,
  )
  expect(h).toEqual({
    series: [{ id: 'scoped:model:Fable', start: 1, resetsAt: 2, points: [[1, 3]] }],
    at: NOW,
  })
  expect(parseHistory('nope', NOW)).toBeNull()
  expect(parseHistory('{"schema":2,"series":[]}', NOW)).toBeNull()
})

test('the bridge is found by statusLine.command alone', () => {
  expect(bridgeInstalled('{"statusLine":{"type":"command","command":"tokenradar statusline --chain"}}')).toBe(true)
  expect(bridgeInstalled('{"statusLine":{"type":"command","command":"ccstatus"}}')).toBe(false)
  expect(bridgeInstalled('{"env":{"X":"tokenradar"}}')).toBe(false)
  expect(bridgeInstalled('{ broken')).toBe(false)
})

test('a reset seen passing raises the notice once; one never seen raises none', () => {
  const start: ResetState = { known: {}, notice: null }
  const soon = NOW + 10 * 60_000
  const a = stepResets(start, [weekly({ resetsAt: soon })], NOW)
  expect(a).toEqual({ known: { seven_day: soon }, notice: null })
  expect(stepResets(a, [weekly({ resetsAt: soon })], NOW + 1)).toBe(a)

  const later = soon + 5 * 60_000
  const b = stepResets(a, [weekly({ resetsAt: soon + 7 * DAY })], later)
  expect(b).toEqual({ known: { seven_day: soon + 7 * DAY }, notice: { id: 'seven_day', at: soon } })
  expect(resetNotice(b, later)).toEqual({ id: 'seven_day', at: soon })
  expect(resetNotice(b, soon + 61 * 60_000)).toBeNull()

  // First seen after its reset: nothing to tell.
  expect(stepResets(start, [weekly({ resetsAt: soon + 7 * DAY })], later).notice).toBeNull()
})

test('the status entry and the /radar summary are short', () => {
  expect(statusText('tr', [weekly()])).toBe('TokenRadar · Haftalık %30 ⏱%70,2')
  expect(statusText('en', [weekly({ id: 'scoped:model:Fable' })])).toBeUndefined()
  expect(summary('en', [weekly()])).toBe('TokenRadar pane opened. Weekly 30% (Claude Code)')
  expect(summary('tr', [])).toBe('TokenRadar paneli açıldı. Henüz limit verisi yok.')
})

test('live sessions are named by name or folder, never by title', () => {
  const r = parseStatus(
    JSON.stringify({
      schema: 1,
      origin: 'app',
      limits: [],
      live_sessions: [
        { session_uuid: 'a', name: null, title: 'secret', project: 'C:\\work\\alpha', cost_usd: 2, share_percent: 4 },
        { session_uuid: 'b', name: ' beta ', title: 'secret', project: '/x/y', status: 'idle' },
      ],
    }),
    'a',
    NOW,
  )
  expect(r?.sessions).toEqual([
    { label: 'alpha', own: true, costUsd: 2, share: 4 },
    { label: 'beta', own: false, costUsd: 0, status: 'idle' },
  ])
  expect(r?.share).toBe(4)
})

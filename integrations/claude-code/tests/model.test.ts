// The pure rules: parsing the CLI's answer, merging, thresholds and text.
// Every value here is synthetic.

import { expect, test } from 'claude-code/testing'

import type { Measure } from '../types'
import { dur, pct } from '../hooks/i18n'
import {
  alertsOf,
  bandLine,
  clock,
  elapsedOf,
  mergeRows,
  parseStatus,
  report,
  rowLevel,
  stepAlerts,
  toastLevel,
  topLevel,
  type Row,
} from '../hooks/model'

const NOW = Date.parse('2026-11-07T12:00:00Z')
const H = 3_600_000

const status = (extra: object = {}) =>
  JSON.stringify({
    schema: 1,
    origin: 'app',
    locale: 'tr',
    limits: [
      {
        kind: 'weekly_all',
        scope: null,
        percent: 60,
        resets_at: NOW + 50 * H,
        source: 'statusline',
        estimated: false,
        observed_at: NOW - 60_000,
        freshness: 'fresh',
        eta_100_at: NOW + 20 * H,
        will_hit_before_reset: true,
        projection_confidence: 'high',
      },
      {
        kind: 'weekly_scoped',
        scope: 'model:Fable',
        percent: 12.5,
        resets_at: NOW + 50 * H,
        source: 'api',
        estimated: false,
        observed_at: NOW - 3 * H,
        freshness: 'stale',
        eta_100_at: null,
        will_hit_before_reset: false,
        projection_confidence: null,
      },
    ],
    absent: [{ kind: 'session', scope: null }],
    live_sessions: [
      { session_uuid: 'other', share_percent: 40 },
      { session_uuid: 's-1', share_percent: 9.25 },
    ],
    future_field: true,
    ...extra,
  })

const row = (over: Partial<Row>): Row => ({
  id: 'seven_day',
  percent: 10,
  source: 'claude_code',
  estimated: false,
  stale: false,
  projected: false,
  willHit: false,
  ...over,
})

test('numbers and durations follow the app formats', () => {
  expect(pct('tr', 72)).toBe('%72')
  expect(pct('tr', 23.5)).toBe('%23,5')
  expect(pct('en', 23.04)).toBe('23%')
  expect(dur('tr', H + 48 * 60_000)).toBe('1 sa 48 dk')
  expect(dur('en', 2 * 24 * H + 3 * H)).toBe('2d 3h')
  expect(dur('en', 2 * H)).toBe('2h')
  expect(dur('tr', -5)).toBe('0 sn')
})

test('the CLI answer is read, its session found, unknown fields ignored', () => {
  const r = parseStatus(status(), 's-1', NOW)
  expect(r?.origin).toBe('app')
  expect(r?.lang).toBe('tr')
  expect(r?.share).toBe(9.25)
  expect(r?.absent).toEqual(['five_hour'])
  expect(r?.limits.map(l => l.id)).toEqual(['seven_day', 'scoped:model:Fable'])
  expect(r?.limits[1]?.stale).toBe(true)
})

test('an older CLI without locale or absent still works; garbage does not', () => {
  const old = JSON.parse(status()) as Record<string, unknown>
  delete old.locale
  delete old.absent
  const r = parseStatus(JSON.stringify(old), 'nobody', NOW)
  expect(r?.lang).toBe('en')
  expect(r?.absent).toEqual([])
  expect(r?.share).toBeUndefined()
  expect(parseStatus('not json', 's-1', NOW)).toBeNull()
  expect(parseStatus(status({ schema: 2 }), 's-1', NOW)).toBeNull()
})

test("Claude Code's figure wins and keeps TokenRadar's projection", () => {
  const m: Measure = {
    windows: [{ kind: 'seven_day', percentUsed: 64, resetsAt: NOW + 50 * H }],
    contextWindow: 200_000,
    at: NOW,
  }
  const rows = mergeRows(m, parseStatus(status(), 's-1', NOW), NOW)
  expect(rows.map(r => r.id)).toEqual(['seven_day', 'scoped:model:Fable'])
  expect(rows[0]).toMatchObject({ percent: 64, source: 'claude_code', willHit: true })
  expect(rows[1]).toMatchObject({ percent: 12.5, source: 'api', stale: true })
})

test('a window past its reset reads ≈ 0 and alerts nothing', () => {
  const m: Measure = {
    windows: [{ kind: 'seven_day', percentUsed: 95, resetsAt: NOW - 1 }],
    contextWindow: 200_000,
    at: NOW,
  }
  const [r] = mergeRows(m, null, NOW)
  expect(r).toMatchObject({ percent: 0, estimated: true })
  expect(alertsOf(mergeRows(m, null, NOW))).toEqual([])
})

test('thresholds are the tray ones: 75 warns, 90 is critical, a projection warns', () => {
  expect(rowLevel(row({ percent: 74.9 }))).toBe(0)
  expect(rowLevel(row({ percent: 75 }))).toBe(1)
  expect(rowLevel(row({ percent: 90 }))).toBe(2)
  expect(rowLevel(row({ percent: 30, willHit: true, etaAt: NOW + H }))).toBe(1)
  const found = alertsOf([
    row({ id: 'scoped:model:Fable', percent: 80 }),
    row({ percent: 92 }),
    row({ id: 'five_hour', percent: 10, willHit: true, etaAt: NOW + H }),
  ])
  expect(found.map(a => [a.row.id, a.level, a.reason])).toEqual([
    ['seven_day', 2, 'used'],
    ['scoped:model:Fable', 1, 'used'],
    ['five_hour', 1, 'projection'],
  ])
})

test('one toast per crossing; a reset starts over', () => {
  let s = { hidden: 0, toasted: 0 } as const as { hidden: 0 | 1 | 2; toasted: 0 | 1 | 2 }
  const seen: boolean[] = []
  for (const level of [0, 1, 1, 2, 2, 1, 0, 1] as const) {
    const step = stepAlerts(s, level)
    seen.push(step.toast)
    s = step.state
  }
  expect(seen).toEqual([false, true, false, true, false, false, false, true])
})

test('the report names sources, absent windows, estimates and the data origin', () => {
  const r = parseStatus(status(), 's-1', NOW)
  const m: Measure = {
    windows: [{ kind: 'seven_day', percentUsed: 64, resetsAt: NOW + 50 * H }],
    contextPercent: 45,
    contextWindow: 200_000,
    costUsd: 4.125,
    at: NOW,
  }
  // Istanbul: three hours ahead of UTC.
  const text = report('tr', mergeRows(m, r, NOW), r, m, NOW, 180)
  expect(text).toBe(
    [
      'TokenRadar · limitler',
      '',
      'Limit             Kullanım  Sıfırlanma  Kaynak                       Bu hızla',
      'Haftalık               %64  2 g 2 sa    Claude Code                  ≈20 sa içinde dolar',
      'Haftalık (Fable)     %12,5  2 g 2 sa    API (3 sa önce güncellendi)  —',
      '5 saatlik: bu hesapta yok.',
      '',
      'Zaman (Haftalık): pencerenin ⏱%70,2 kadarı geçti, kullanılan %64; zaman çizgisinin 6 puan (10 sa 28 dk) gerisinde.',
      'Zaman çizgisi: Cmt 17:00 %71,4 · Paz 17:00 %85,7 · Pzt 17:00 sıfırlanma',
      '',
      'Bu oturumun 5 saatlik penceredeki payı: ≈%9,3',
      'Bağlam penceresi: %45 dolu (200.000 token)',
      'Oturum maliyeti: API $4,13 (API fiyatıyla, fatura değil)',
      '',
      'Veri: Claude Code ve TokenRadar uygulamasından canlı veri.',
      '≈ işareti ölçümü değil tahmini gösterir.',
    ].join('\n'),
  )
})

test('without TokenRadar the report says so and shows Claude Code alone', () => {
  const m: Measure = {
    windows: [{ kind: 'seven_day', percentUsed: 21 }],
    contextWindow: 200_000,
    at: NOW,
  }
  const text = report('en', mergeRows(m, null, NOW), null, m, NOW)
  expect(text).toContain('Weekly   21%  —          Claude Code  —')
  expect(text).toContain('Data: Claude Code only.')
  expect(text).not.toContain('≈ marks')
})

test('elapsed time comes from the reset and the window length', () => {
  expect(elapsedOf('seven_day', NOW + 84 * H, NOW)).toBe(50)
  expect(elapsedOf('scoped:model:Fable', NOW + 168 * H, NOW)).toBe(0)
  expect(elapsedOf('five_hour', NOW + H, NOW)).toBe(80)
  // Past the reset, beyond the window, or a window of unknown length: no reading.
  expect(elapsedOf('five_hour', NOW, NOW)).toBeUndefined()
  expect(elapsedOf('five_hour', NOW + 6 * H, NOW)).toBeUndefined()
  expect(elapsedOf('other:spend_limit', NOW + H, NOW)).toBeUndefined()
  const m: Measure = {
    windows: [{ kind: 'seven_day', percentUsed: 21, resetsAt: NOW + 84 * H }],
    contextWindow: 200_000,
    at: NOW,
  }
  expect(mergeRows(m, null, NOW)[0]?.elapsed).toBe(50)
})

test('use ten points ahead of time shows on the band but never toasts', () => {
  const ahead = row({ percent: 60, elapsed: 50 })
  expect(alertsOf([row({ percent: 59.9, elapsed: 50 })])).toEqual([])
  expect(alertsOf([row({ percent: 60, elapsed: 50, estimated: true })])).toEqual([])
  const found = alertsOf([ahead])
  expect(found.map(a => [a.level, a.reason])).toEqual([[1, 'pace']])
  expect(topLevel(found)).toBe(1)
  expect(toastLevel(found)).toBe(0)
  expect(bandLine('tr', found[0]!, NOW)).toBe(
    'Haftalık kullanım zamanın 10 puan önünde: kullanılan %60, geçen zaman ⏱%50 · Claude Code',
  )
  expect(bandLine('en', found[0]!, NOW)).toBe(
    'Weekly use 10 points ahead of time: 60% used, ⏱50% of the window gone · Claude Code',
  )
  // A threshold alert beside it still toasts.
  expect(toastLevel(alertsOf([ahead, row({ id: 'five_hour', percent: 80 })]))).toBe(1)
})

test('a pace alert keeps the band hidden and raises no toast', () => {
  const pace = stepAlerts({ hidden: 1, toasted: 0 }, 1, 0)
  expect(pace).toEqual({ state: { hidden: 1, toasted: 0 }, toast: false })
  // Crossing 75 % afterwards still toasts once.
  expect(stepAlerts(pace.state, 1, 1).toast).toBe(true)
})

test('the time line reads clock times at the given offset', () => {
  expect(clock('tr', NOW, 180)).toBe('Cmt 15:00')
  expect(clock('en', NOW, -300)).toBe('Sat 07:00')
  expect(clock('en', NOW, -780)).toBe('Fri 23:00')
})

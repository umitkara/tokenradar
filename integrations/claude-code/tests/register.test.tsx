// The hooks end to end: band, toast, hide, /radar and the CLI's pace.
// The hooks registered on `on` stand for the engine; every value is synthetic.

import { expect, mock, test } from 'claude-code/testing'
import type { On, SessionMeasureInput } from 'claude-code'

const NOW = Date.parse('2026-11-07T12:00:00Z')
const H = 3_600_000
const RESETS = new Date(NOW + 50 * H).toISOString()

const BAND = {
  plugin: 'tokenradar',
  component: 'AbovePrompt',
  props: {
    hasSurvey: false,
    isWorking: false,
    maxRows: 10,
    bodyColumns: 100,
    scroll: { offset: 0, bodyRows: 10 },
    view: {},
  },
} as const

const RADAR = {
  command: 'radar',
  args: '',
  origin: { kind: 'composer' },
  presentation: { isFullscreen: false, columns: 120 },
} as const

type World = { toasts: string[]; cliRuns: () => number; clock: ReturnType<typeof mock.clock> }

/** The engine beneath the mod; `cli` is the TokenRadar answer, `null` for "not found". */
function world(on: On, cli: { origin: 'app' | 'database'; locale?: string } | null): World {
  const clock = mock.clock(on, { now: NOW })
  const toasts: string[] = []
  let runs = 0
  on('session.id', () => ({ value: 's-1' }))
  on('session.start', (_, e) => ({ cwd: e.cwd }))
  on('session.measure', (_, e) => ({ changed: e.changed }))
  on('command.register', (_, e) => ({ value: { command: e.name } }))
  on('session.usage', () => ({
    value: {
      startedAt: NOW - H,
      context: { window: 200_000, percent: 12 },
      rateLimits: [{ kind: 'seven_day', percentUsed: 30, resetsAt: RESETS }],
      cost: { usd: 1.5 },
    },
  }))
  on('ui.toast', (_, e) => {
    toasts.push(e.text)
    return { value: undefined }
  })
  on('process.run', () => {
    runs += 1
    if (cli === null) return { deny: 'not found' }
    return {
      value: {
        exitCode: 0,
        stdout: JSON.stringify({
          schema: 1,
          origin: cli.origin,
          locale: cli.locale,
          limits: [
            {
              kind: 'weekly_scoped',
              scope: 'model:Fable',
              percent: 5,
              resets_at: NOW + 50 * H,
              source: 'api',
              estimated: false,
              observed_at: NOW,
              freshness: 'fresh',
              eta_100_at: null,
              will_hit_before_reset: false,
              projection_confidence: null,
            },
          ],
          absent: [{ kind: 'session', scope: null }],
          live_sessions: [{ session_uuid: 's-1', share_percent: null }],
        }),
        stderr: '',
        isStdoutTruncated: false,
        isStderrTruncated: false,
      },
    }
  })
  // The engine's own band: what shows when the mod draws nothing.
  on('ui.render', ($, e) => {
    const { Text } = $.ui.resolve(e)
    return <Text>engine</Text>
  })
  return { toasts, cliRuns: () => runs, clock }
}

const weekly = (percentUsed: number, resetsAt = RESETS): SessionMeasureInput => ({
  context: { window: 200_000, percent: 20 },
  rateLimits: [{ kind: 'seven_day', percentUsed, resetsAt }],
  changed: ['rateLimits'],
})

test('below the threshold the band draws nothing and no toast is raised', async ($, on) => {
  const w = world(on, { origin: 'database' })
  await $.session.start({ cwd: '/', surface: 'terminal', isInteractive: true })
  await $.session.measure(weekly(60))
  for (const surface of ['terminal', 'desktop'] as const) {
    const ui = await $.ui.mount({ ...BAND, surface })
    expect((await ui.find({ type: 'Text' }))?.text).toBe('engine')
    await ui.unmount()
  }
  expect(w.toasts).toEqual([])
})

test('crossing a threshold shows the band and toasts once per level', async ($, on) => {
  const w = world(on, { origin: 'database', locale: 'tr' })
  await $.session.start({ cwd: '/', surface: 'terminal', isInteractive: true })
  await $.session.measure(weekly(60))
  await $.session.measure(weekly(82))
  await $.session.measure(weekly(83))
  expect(w.toasts).toEqual(['TokenRadar: Haftalık limit %82 seviyesinde'])

  const ui = await $.ui.mount({ ...BAND, surface: 'terminal' })
  const line = await ui.find({ type: 'Text', text: /Haftalık/ })
  expect(line?.text).toBe('Haftalık limit: %83 kullanıldı · 2 g 2 sa içinde sıfırlanır · Claude Code')
  await ui.unmount()

  await $.session.measure(weekly(91))
  expect(w.toasts).toEqual([
    'TokenRadar: Haftalık limit %82 seviyesinde',
    'TokenRadar: Haftalık limit %91 seviyesinde',
  ])
})

test('use ahead of the time line shows on the band without a toast', async ($, on) => {
  const w = world(on, { origin: 'database', locale: 'tr' })
  await $.session.start({ cwd: '/', surface: 'terminal', isInteractive: true })
  // Two of the seven days gone (≈ 28.6 % of the time), 45 % used.
  await $.session.measure(weekly(45, new Date(NOW + 120 * H).toISOString()))
  expect(w.toasts).toEqual([])
  const ui = await $.ui.mount({ ...BAND, surface: 'terminal' })
  expect((await ui.find({ type: 'Text', text: /Haftalık/ }))?.text).toBe(
    'Haftalık kullanım zamanın 16 puan önünde: kullanılan %45, geçen zaman ⏱%28,6 · Claude Code',
  )
  await ui.unmount()
})

test('while the desktop app runs, it notifies and the mod does not', async ($, on) => {
  const w = world(on, { origin: 'app' })
  await $.session.start({ cwd: '/', surface: 'terminal', isInteractive: true })
  await $.session.measure(weekly(95))
  expect(w.toasts).toEqual([])
  const ui = await $.ui.mount({ ...BAND, surface: 'desktop' })
  expect((await ui.find({ type: 'Text', text: /Weekly/ }))?.text).toBe(
    'Weekly 95% used · resets in 2d 2h · Claude Code',
  )
  await ui.unmount()
})

test('hide keeps the band away until the level rises', async ($, on) => {
  world(on, { origin: 'app' })
  await $.session.start({ cwd: '/', surface: 'terminal', isInteractive: true })
  await $.session.measure(weekly(80))

  const ui = await $.ui.mount({ ...BAND, surface: 'terminal' })
  await ui.press({ key: 'hide' })
  expect((await ui.find({ type: 'Text' }))?.text).toBe('engine')
  await ui.unmount()

  await $.session.measure(weekly(85))
  const same = await $.ui.mount({ ...BAND, surface: 'terminal' })
  expect((await same.find({ type: 'Text' }))?.text).toBe('engine')
  await same.unmount()

  await $.session.measure(weekly(92))
  const back = await $.ui.mount({ ...BAND, surface: 'terminal' })
  expect((await back.find({ type: 'Text', text: /Weekly/ }))?.text).toContain('92%')
  await back.unmount()
})

test('the CLI runs at most once per 30 s of measurements', async ($, on) => {
  const w = world(on, { origin: 'app' })
  await $.session.start({ cwd: '/', surface: 'terminal', isInteractive: true })
  await $.session.measure(weekly(10))
  await $.session.measure(weekly(11))
  expect(w.cliRuns()).toBe(1)
  await w.clock.advance(30_000)
  await $.session.measure(weekly(12))
  expect(w.cliRuns()).toBe(2)
})

test('/radar works without TokenRadar and says so', async ($, on) => {
  world(on, null)
  await $.session.start({ cwd: '/', surface: 'terminal', isInteractive: true })
  const { text } = await $.command.run(RADAR)
  expect(text).toContain('Weekly   30%  2d 2h      Claude Code  —')
  expect(text).toContain('Data: Claude Code only.')
  expect(text).toContain('Session cost: API $1.50')
})

test('/radar adds what only TokenRadar knows, in its language', async ($, on) => {
  world(on, { origin: 'app', locale: 'tr' })
  await $.session.start({ cwd: '/', surface: 'terminal', isInteractive: true })
  const { text } = await $.command.run(RADAR)
  expect(text).toContain('Haftalık (Fable)')
  expect(text).toContain('5 saatlik: bu hesapta yok.')
  expect(text).toContain('Veri: Claude Code ve TokenRadar uygulamasından canlı veri.')
})

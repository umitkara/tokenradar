// The hooks end to end: band, toast, hide, reset line, status entry, the pane on
// every surface, /radar and the CLI's pace. The hooks registered on `on` stand for
// the engine; every value is synthetic.

import { expect, mock, test } from 'claude-code/testing'
import type { On, SessionMeasureInput } from 'claude-code'

const NOW = Date.parse('2026-11-07T12:00:00Z')
const H = 3_600_000
const RESETS_AT = NOW + 50 * H
const RESETS = new Date(RESETS_AT).toISOString()

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

const PANE = {
  plugin: 'tokenradar',
  component: 'Pane',
  requestId: 'tokenradar',
  props: {
    title: 'TokenRadar',
    isFocused: false,
    bodyColumns: 56,
    placement: 'dock',
    scroll: { offset: 0, bodyRows: 40 },
    view: {},
  },
} as const

const RADAR = {
  command: 'radar',
  args: '',
  origin: { kind: 'composer' },
  presentation: { isFullscreen: false, columns: 120 },
} as const

const SURFACES = ['terminal', 'desktop', 'vscode', 'mobile'] as const

type Options = {
  origin: 'app' | 'database'
  locale?: string
  /** What `$.ui.open` answers; `true` by default. */
  placed?: boolean
  /** Claude Code's settings.json; absent, the file is missing. */
  settings?: string
}

type World = {
  toasts: string[]
  statuses: (string | undefined)[]
  opened: string[]
  reads: string[]
  cliRuns: () => number
  historyRuns: () => number
  clock: ReturnType<typeof mock.clock>
}

const statusJson = (o: Options) =>
  JSON.stringify({
    schema: 1,
    origin: o.origin,
    locale: o.locale,
    limits: [
      {
        kind: 'weekly_scoped',
        scope: 'model:Fable',
        percent: 5,
        resets_at: RESETS_AT,
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
    live_sessions: [
      { session_uuid: 's-1', name: null, project: 'D:/work/alpha', share_percent: null, cost_usd: 3 },
      {
        session_uuid: 's-2',
        name: 'beta',
        title: 'A TITLE FROM THE CONVERSATION',
        project: 'D:/work/beta',
        status: 'idle',
        share_percent: null,
        cost_usd: 1,
      },
    ],
  })

const historyJson = JSON.stringify({
  schema: 1,
  generated_at: NOW,
  series: [
    {
      kind: 'weekly_all',
      scope: null,
      window_start: RESETS_AT - 168 * H,
      resets_at: RESETS_AT,
      points: [
        { at: RESETS_AT - 160 * H, percent: 2, source: 'statusline' },
        { at: RESETS_AT - 100 * H, percent: 14, source: 'statusline' },
        { at: NOW - H, percent: 29, source: 'api' },
      ],
    },
  ],
})

/** The engine beneath the mod; `o === null` is "TokenRadar not found". */
function world(on: On, o: Options | null): World {
  const clock = mock.clock(on, { now: NOW })
  const toasts: string[] = []
  const statuses: (string | undefined)[] = []
  const opened: string[] = []
  const reads: string[] = []
  let runs = 0
  let historyRuns = 0
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
  on('ui.status', (_, e) => {
    statuses.push(e.text)
    return { value: undefined }
  })
  on('ui.open', (_, e) => {
    opened.push(e.id)
    return { value: o?.placed === false ? { isPlaced: false, reason: 'narrow' } : { isPlaced: true } }
  })
  on('ui.close', () => ({ value: undefined }))
  on('env.get', (_, e) => ({ value: e.name === 'USERPROFILE' ? 'C:/Users/u' : undefined }))
  on('fs.read', (_, e) => {
    // The engine hands the path on in the platform's own spelling.
    const path = e.path.replaceAll('\\', '/')
    reads.push(path)
    if (path === 'C:/Users/u/.claude/settings.json' && o?.settings !== undefined) {
      return { value: o.settings }
    }
    return { deny: 'ENOENT' }
  })
  on('process.run', (_, e) => {
    if (o === null) return { deny: 'not found' }
    const isHistory = e.argv[1] === 'history'
    if (isHistory) historyRuns += 1
    else runs += 1
    return {
      value: {
        exitCode: 0,
        stdout: isHistory ? historyJson : statusJson(o),
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
  return { toasts, statuses, opened, reads, cliRuns: () => runs, historyRuns: () => historyRuns, clock }
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
  for (const surface of SURFACES) {
    const ui = await $.ui.mount({ ...BAND, surface })
    expect((await ui.find({ type: 'Text' }))?.text).toBe('engine')
    await ui.unmount()
  }
  expect(w.toasts).toEqual([])
})

test('crossing a threshold shows the one-line band and toasts once per level', async ($, on) => {
  const w = world(on, { origin: 'database', locale: 'tr' })
  await $.session.start({ cwd: '/', surface: 'terminal', isInteractive: true })
  await $.session.measure(weekly(60))
  await $.session.measure(weekly(82))
  await $.session.measure(weekly(83))
  expect(w.toasts).toEqual(['TokenRadar: Haftalık limit %82 seviyesinde'])

  for (const surface of SURFACES) {
    const ui = await $.ui.mount({ ...BAND, surface })
    const line = await ui.find({ type: 'Text', text: /Haftalık/ })
    // The mini bar: 83 % filled, the time mark at ⏱70 %.
    expect(line?.text).toBe(
      '███████│░░ Haftalık limit: %83 kullanıldı · 2 g 2 sa içinde sıfırlanır · Claude Code',
    )
    expect((await ui.findAll({ type: 'Button' })).map(b => b.key)).toEqual(['detail', 'hide'])
    await ui.unmount()
  }

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
  expect((await ui.find({ type: 'Text', text: /Haftalık/ }))?.text).toContain(
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
  expect((await ui.find({ type: 'Text', text: /Weekly/ }))?.text).toContain(
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

test("the band's details button opens the pane", async ($, on) => {
  const w = world(on, { origin: 'app' })
  await $.session.start({ cwd: '/', surface: 'terminal', isInteractive: true })
  await $.session.measure(weekly(80))
  const ui = await $.ui.mount({ ...BAND, surface: 'terminal' })
  await ui.press({ key: 'detail' })
  await ui.unmount()
  expect(w.opened).toEqual(['tokenradar'])
  expect(w.historyRuns()).toBe(1)
})

test('a window that resets shows the reset line once, until OK', async ($, on) => {
  const w = world(on, { origin: 'app' })
  await $.session.start({ cwd: '/', surface: 'terminal', isInteractive: true })
  const soon = NOW + 10 * 60_000
  await $.session.measure(weekly(40, new Date(soon).toISOString()))
  // Past the reset, with Claude Code's new window.
  await w.clock.advance(15 * 60_000)
  await $.session.measure(weekly(1, new Date(soon + 168 * H).toISOString()))

  const ui = await $.ui.mount({ ...BAND, surface: 'terminal' })
  expect((await ui.find({ type: 'Text', text: /reset/ }))?.text).toBe('✓ Weekly limit reset')
  await ui.press({ key: 'reset-ok' })
  expect((await ui.find({ type: 'Text' }))?.text).toBe('engine')
  await ui.unmount()
})

test('the status entry shows only where the statusline bridge is not installed', async ($, on) => {
  const w = world(on, { origin: 'app' })
  await $.session.start({ cwd: '/', surface: 'terminal', isInteractive: true })
  await $.session.measure(weekly(30))
  expect(w.statuses).toEqual(['TokenRadar · Weekly 30% ⏱70.2%'])
})

test('with the bridge in settings.json the mod sets no status entry', async ($, on) => {
  const settings = JSON.stringify({ statusLine: { type: 'command', command: 'tokenradar statusline' } })
  const w = world(on, { origin: 'app', settings })
  await $.session.start({ cwd: '/', surface: 'terminal', isInteractive: true })
  await $.session.measure(weekly(30))
  expect(w.reads).toContain('C:/Users/u/.claude/settings.json')
  expect(w.statuses).toEqual([])
})

test('the pane draws cards, strip, chart, sessions and context on every surface', async ($, on) => {
  world(on, { origin: 'app', locale: 'tr' })
  await $.session.start({ cwd: '/', surface: 'terminal', isInteractive: true })
  await $.session.measure(weekly(30))
  await $.command.run(RADAR)

  for (const surface of SURFACES) {
    const ui = await $.ui.mount({ ...PANE, surface })
    const texts = (await ui.findAll({ type: 'Text' })).map(t => t.text)
    const all = texts.join('\n')
    // Cards with source badges, the time mark and the reset.
    expect(texts).toContain('Haftalık')
    expect(texts).toContain(' Claude Code ')
    expect(texts).toContain(' API ')
    expect(all).toContain('%30 ⏱%70,2')
    expect(all).toContain('2 g 2 sa içinde sıfırlanır')
    expect(all).toContain('5 saatlik: bu hesapta yok.')
    // The weekly strip: title, now pointer, end-of-day time line.
    expect(texts).toContain('Zaman şeridi · Haftalık')
    expect(all).toContain('▲ şimdi')
    expect(all).toContain('%100')
    // Live sessions by name or folder, never by title; this one first.
    expect(all).toContain('● bu oturum')
    expect(all).toContain('beta')
    expect(all).not.toContain('TITLE')
    expect(all).toContain('API $3,00')
    // Context.
    expect(all).toContain('%12 · 200.000 token')
    // The chart: a Raster in the terminal, an Svg elsewhere.
    const raster = await ui.find({ type: 'Raster' })
    const svg = await ui.find({ type: 'Svg' })
    if (surface === 'terminal') {
      expect(svg).toBeUndefined()
      expect(raster?.props.rows).toBe(6)
      expect(raster?.props.columns).toBe(56)
    } else {
      expect(raster).toBeUndefined()
      expect(String(svg?.props.source)).toContain('<svg')
      expect(String(svg?.props.alt)).toContain('Haftalık')
    }
    await ui.unmount()
  }
})

test('the CLI runs at most once per 30 s of measurements', async ($, on) => {
  const w = world(on, { origin: 'app' })
  await $.session.start({ cwd: '/', surface: 'terminal', isInteractive: true })
  await $.session.measure(weekly(10))
  await $.session.measure(weekly(11))
  expect(w.cliRuns()).toBe(1)
  expect(w.historyRuns()).toBe(0)
  await w.clock.advance(30_000)
  await $.session.measure(weekly(12))
  expect(w.cliRuns()).toBe(2)
})

test('/radar opens the pane and answers in one line', async ($, on) => {
  const w = world(on, { origin: 'app', locale: 'tr' })
  await $.session.start({ cwd: '/', surface: 'terminal', isInteractive: true })
  const { text } = await $.command.run(RADAR)
  expect(w.opened).toEqual(['tokenradar'])
  expect(text).toBe(
    'TokenRadar paneli açıldı. Haftalık %30 (Claude Code) · Haftalık (Fable) %5 (API)',
  )
})

test('/radar without a pane works without TokenRadar and says so', async ($, on) => {
  world(on, null)
  await $.session.start({ cwd: '/', surface: 'terminal', isInteractive: true })
  const { text } = await $.command.run(RADAR)
  // With no CLI, `ui.open` still answers placed; this world places it.
  expect(text).toBe('TokenRadar pane opened. Weekly 30% (Claude Code)')
})

test('/radar falls back to the full report where the pane is not placed', async ($, on) => {
  world(on, { origin: 'app', locale: 'tr', placed: false })
  await $.session.start({ cwd: '/', surface: 'terminal', isInteractive: true })
  const { text } = await $.command.run(RADAR)
  expect(text).toContain('Haftalık (Fable)')
  expect(text).toContain('5 saatlik: bu hesapta yok.')
  expect(text).toContain('Veri: Claude Code ve TokenRadar uygulamasından canlı veri.')
})

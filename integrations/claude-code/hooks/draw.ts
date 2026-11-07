// Pure drawing of the mod's visual surfaces (ADR-046): bars and the time strip as
// colored text runs every surface draws, the usage chart as SVG markup for the
// desktop, VS Code and mobile, and as Raster cells for the terminal. Nothing here
// touches `$`, so the tests drive it directly. Nothing moves on its own (ADR-019).

import type { Level, Series } from '../types'
import { pct, t, type Lang } from './i18n'
import { DAY_MS, clock, type Row } from './model'

/** What a run of cells means; the surface picks the color (`toneColor`). */
export type Tone = 'ok' | 'warn' | 'crit' | 'track' | 'mark'
export type Seg = { text: string; tone: Tone }

export const levelTone = (level: Level): Tone =>
  level === 2 ? 'crit' : level === 1 ? 'warn' : 'ok'

/**
 * Text colors: theme keys where the theme has the meaning (warning, error, the dim
 * track), the statusline's cyan for the normal state. A remote surface gets the
 * design's mid-lightness cyan, readable on light and dark alike.
 */
export function toneColor(tone: Tone, surface: string): string {
  switch (tone) {
    case 'ok':
      return surface === 'terminal' ? 'cyan' : '#00afaf'
    case 'warn':
      return 'warning'
    case 'crit':
      return 'error'
    case 'track':
      return 'subtle'
    case 'mark':
      return 'text'
  }
}

function merge(cells: readonly Seg[]): Seg[] {
  const out: Seg[] = []
  for (const c of cells) {
    const last = out[out.length - 1]
    if (last !== undefined && last.tone === c.tone) last.text += c.text
    else out.push({ ...c })
  }
  return out
}

const clampInt = (n: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, Math.floor(n)))

/**
 * A bar `width` cells wide: the used share filled (`▒` for an estimate, the
 * design's dashed look), the rest track, and the elapsed time marked `│` in its
 * cell, as the statusline does (ADR-044).
 */
export function bar(
  percent: number,
  elapsed: number | undefined,
  width: number,
  tone: Tone,
  estimated = false,
): Seg[] {
  const w = Math.max(1, Math.floor(width))
  const filled = clampInt(Math.round((percent / 100) * w), 0, w)
  const mark = elapsed === undefined ? -1 : clampInt((elapsed / 100) * w, 0, w - 1)
  const cells: Seg[] = []
  for (let i = 0; i < w; i++) {
    if (i === mark) cells.push({ text: '│', tone: 'mark' })
    else if (i < filled) cells.push({ text: estimated ? '▒' : '█', tone })
    else cells.push({ text: '░', tone: 'track' })
  }
  return merge(cells)
}

/**
 * The weekly time strip, both views at once: calendar days (names, `┊` at
 * midnight) over the use, the 24-hour periods (`│` at each reset hour, with
 * the time line's value below), "now" and the reset's day and hour.
 */
export type Strip = { labels: string; cells: Seg[]; pointer: string; periods: string; reset: string }

const isWeekly = (id: string) => id === 'seven_day' || id.startsWith('scoped:')

/**
 * The local `step` boundaries (midnights for days, whole hours for hours)
 * strictly inside `start`..`end`. A fixed offset: a DST change inside the
 * window shifts the later ones by its hour.
 */
export function boundaries(start: number, end: number, step: number, tzOffsetMin: number): number[] {
  const off = tzOffsetMin * 60_000
  const out: number[] = []
  for (let at = Math.floor((start + off) / step) * step + step - off; at < end; at += step) {
    if (at > start) out.push(at)
  }
  return out
}

/**
 * The weekly window as one strip of calendar days: boundaries (`┊`) at local
 * midnight, so the first and last days are partial (the window starts and ends
 * at the reset hour). Day names stand at each day's start; the first says the
 * reset hour when it has room. Below, the 24-hour periods from the reset.
 */
export function strip(
  lang: Lang,
  row: Row,
  tone: Tone,
  width: number,
  tzOffsetMin: number,
): Strip | null {
  if (!isWeekly(row.id) || row.resetsAt === undefined || row.elapsed === undefined) return null
  if (width < 35) return null
  const w = width
  const length = 7 * DAY_MS
  const start = row.resetsAt - length
  const at = (c: number) => Math.round(((c - start) / length) * w)
  const cuts = boundaries(start, row.resetsAt, DAY_MS, tzOffsetMin)
  const cols = cuts.map(at)

  // Labels, left to right, each only where it fits before the next day.
  const label = Array<string>(w).fill(' ')
  const put = (line: string[], text: string, from: number) => {
    for (let i = 0; i < text.length && from + i < w; i++) line[from + i] = text[i]!
  }
  const starts = [0, ...cols]
  starts.forEach((from, i) => {
    const room = (starts[i + 1] ?? w + 1) - from - 1
    const [day = '', hm = ''] = clock(lang, i === 0 ? start : cuts[i - 1]!, tzOffsetMin).split(' ')
    const text = i === 0 && `${day} ${hm}`.length <= room ? `${day} ${hm}` : day
    if (text.length <= room) put(label, text, from)
  })

  // The 24-hour periods: a solid boundary (`│`) at each reset hour on the
  // strip, and below it the time line's value there (k/7 of the window),
  // centred, where it keeps a space from the previous one.
  const periodCols: number[] = []
  const period = Array<string>(w).fill(' ')
  let free = 0
  for (let k = 1; k < 7; k++) {
    const c = Math.round((k / 7) * w)
    periodCols.push(c)
    const text = pct(lang, Math.round((k / 7) * 100))
    const from = c - Math.floor(text.length / 2)
    if (from >= free && from + text.length <= w) {
      put(period, text, from)
      free = from + text.length + 1
    }
  }

  const filled = clampInt(Math.round((row.percent / 100) * w), 0, w)
  const cells: Seg[] = []
  for (let i = 0; i < w; i++) {
    if (i < filled) cells.push({ text: row.estimated ? '▒' : '█', tone })
    else cells.push({ text: periodCols.includes(i) ? '│' : cols.includes(i) ? '┊' : '░', tone: 'track' })
  }

  const col = clampInt((row.elapsed / 100) * w, 0, w - 1)
  const word = t(lang).now
  const pointer =
    col + 2 + word.length <= w
      ? `${' '.repeat(col)}▲ ${word}`
      : `${' '.repeat(Math.max(0, col - word.length - 1))}${word} ▲`

  return {
    labels: label.join('').trimEnd(),
    cells: merge(cells),
    pointer,
    periods: period.join('').trimEnd(),
    reset: clock(lang, row.resetsAt, tzOffsetMin),
  }
}

// --- The chart -----------------------------------------------------------

/** The series' readings up to now, then the row's current value now. */
function curve(series: Series, row: Row, now: number): [number, number][] {
  const pts = series.points.filter(([at]) => at >= series.start && at <= now)
  const last = pts[pts.length - 1]
  if (last === undefined || last[0] < now) pts.push([now, row.percent])
  return pts
}

/** Gridline instants: each day of a week, each hour of the 5-hour window. */
function grid(series: Series, tzOffsetMin: number): number[] {
  const step = series.resetsAt - series.start > DAY_MS ? DAY_MS : 3_600_000
  return boundaries(series.start, series.resetsAt, step, tzOffsetMin)
}

const SVG_W = 700
const SVG_H = 170
const TOP = 8
const PLOT_H = 136

/**
 * The usage curve over the window against the time line (dashed diagonal), with
 * day gridlines, the 75/90 thresholds and a "now" line. Colors follow the
 * design's light and dark tokens through the viewer's color scheme.
 */
export function chartSvg(
  lang: Lang,
  series: Series,
  row: Row,
  level: Level,
  now: number,
  tzOffsetMin: number,
): string {
  const length = series.resetsAt - series.start
  const x = (at: number) => (((at - series.start) / length) * SVG_W).toFixed(1)
  const y = (p: number) => (TOP + (1 - Math.min(Math.max(p, 0), 100) / 100) * PLOT_H).toFixed(1)
  const pts = curve(series, row, now)
  const line = pts.map(([at, p], i) => `${i === 0 ? 'M' : 'L'}${x(at)},${y(p)}`).join(' ')
  const first = pts[0]
  const area =
    first === undefined ? '' : `${line} L${x(now)},${y(0)} L${x(first[0])},${y(0)} Z`
  const tone = levelTone(level)
  const weekly = length > DAY_MS
  const cuts = grid(series, tzOffsetMin)
  const lines = cuts
    .map(at => `<line class="g" x1="${x(at)}" y1="${TOP}" x2="${x(at)}" y2="${y(0)}"/>`)
    .join('')
  // Calendar days: the first one starts at the reset hour, so it says so when
  // its part of the width has room for it.
  const starts = [series.start, ...cuts]
  const labels = starts
    .map((at, i) => {
      const [day, hm] = clock(lang, at, tzOffsetMin).split(' ')
      const room = Number(x(starts[i + 1] ?? series.resetsAt)) - Number(x(at))
      const text = !weekly ? hm : i === 0 && room >= 64 ? `${day} ${hm}` : room >= 28 ? day : ''
      return text === '' ? '' : `<text x="${(Number(x(at)) + 4).toFixed(1)}" y="${SVG_H - 6}">${text}</text>`
    })
    .join('')
  const dash = row.estimated ? ' stroke-dasharray="6 4"' : ''
  return (
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${SVG_W} ${SVG_H}" width="${SVG_W}" height="${SVG_H}">` +
    '<style>' +
    '.g{stroke:#ccd2d7;stroke-width:1}.t{stroke:#7c8186;stroke-width:1.5;stroke-dasharray:5 5;fill:none}' +
    '.w{stroke:#9e5800;opacity:.45;stroke-dasharray:2 4}.c{stroke:#be222a;opacity:.45;stroke-dasharray:2 4}' +
    '.n{stroke:#5f6368;stroke-width:1}.ok{stroke:#00757c;fill:#00757c}.warn{stroke:#9e5800;fill:#9e5800}' +
    '.crit{stroke:#be222a;fill:#be222a}text{font:11px system-ui,sans-serif;fill:#5f6368}' +
    '@media (prefers-color-scheme:dark){.g{stroke:#494e52}.n{stroke:#c4c8cc}text{fill:#c4c8cc}' +
    '.ok{stroke:#3adfd7;fill:#3adfd7}.warn{stroke:#fdbe45;fill:#fdbe45}.crit{stroke:#ff6f69;fill:#ff6f69}' +
    '.w{stroke:#fdbe45}.c{stroke:#ff6f69}}' +
    '</style>' +
    lines +
    `<line class="w" x1="0" y1="${y(75)}" x2="${SVG_W}" y2="${y(75)}"/>` +
    `<line class="c" x1="0" y1="${y(90)}" x2="${SVG_W}" y2="${y(90)}"/>` +
    `<line class="t" x1="0" y1="${y(0)}" x2="${SVG_W}" y2="${y(100)}"/>` +
    (area === '' ? '' : `<path class="${tone}" d="${area}" stroke="none" fill-opacity=".18"/>`) +
    (line === '' ? '' : `<path class="${tone}" d="${line}" fill="none" stroke-width="2.5"${dash}/>`) +
    `<line class="n" x1="${x(now)}" y1="${TOP}" x2="${x(now)}" y2="${y(0)}"/>` +
    labels +
    '</svg>'
  )
}

/** The design's mid-lightness signals (readable on light and dark terminals), 0xRRGGBB. */
const RASTER_TONE: Record<'ok' | 'warn' | 'crit', number> = {
  ok: 0x00afaf,
  warn: 0xdd9314,
  crit: 0xe54c4a,
}
const RASTER_TRACK = 0x7c8186
const DEFAULT_COLOR = 0x01000000
const BLOCKS = [0x20, 0x2581, 0x2582, 0x2583, 0x2584, 0x2585, 0x2586, 0x2587, 0x2588]

/** The time area's shade: dimmer than the track, readable on light and dark terminals. */
const RASTER_TIME = 0x5a5f64

/**
 * The chart as Raster cells (terminal), drawn like the bars: in each column the
 * time gone (the time line, up to now) as a gray column of eighth blocks, the
 * use in front of it in the level's color, day boundaries as dotted columns above both, in
 * `columns` × `rows` cells, base64 as `RasterProps.cells` takes them. A cell
 * where the use ends below the time keeps the time's gray as its background.
 */
export function chartCells(
  series: Series,
  row: Row,
  level: Level,
  now: number,
  columns: number,
  rows: number,
  tzOffsetMin: number,
): string {
  const length = series.resetsAt - series.start
  const pts = curve(series, row, now)
  const fg = RASTER_TONE[levelTone(level) as 'ok' | 'warn' | 'crit']
  const span = length / columns
  const cuts = grid(series, tzOffsetMin).map(at => Math.floor((at - series.start) / span))
  const words = new Uint32Array(columns * rows * 3)
  const eighths = (p: number) => Math.round((Math.min(Math.max(p, 0), 100) / 100) * rows * 8)
  let p = 0
  let used = 0
  for (let c = 0; c < columns; c++) {
    const end = series.start + (c + 1) * span
    const past = series.start + c * span <= now
    while (p < pts.length && pts[p]![0] <= Math.min(end, now)) used = pts[p++]![1]
    const u = past ? eighths(used) : 0
    const time = past ? eighths(((c + 1) / columns) * 100) : 0
    for (let r = 0; r < rows; r++) {
      const bottom = (rows - 1 - r) * 8
      const nu = Math.min(8, Math.max(0, u - bottom))
      const nt = Math.min(8, Math.max(0, time - bottom))
      let glyph = 0x20
      let color = DEFAULT_COLOR
      let back = DEFAULT_COLOR
      if (nu > 0) {
        glyph = BLOCKS[nu]!
        color = fg
        if (nu < 8 && nt === 8) back = RASTER_TIME
      } else if (nt > 0) {
        glyph = BLOCKS[nt]!
        color = RASTER_TIME
      } else if (cuts.includes(c)) {
        glyph = 0x250a
        color = RASTER_TRACK
      }
      const i = (r * columns + c) * 3
      words[i] = glyph
      words[i + 1] = color
      words[i + 2] = back
    }
  }
  return base64(new Uint8Array(words.buffer))
}

const B64 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'

/** Standard padded base64 (the module has no `btoa` it can count on). */
export function base64(bytes: Uint8Array): string {
  let out = ''
  for (let i = 0; i < bytes.length; i += 3) {
    const a = bytes[i]!
    const b = bytes[i + 1]
    const c = bytes[i + 2]
    const n = (a << 16) | ((b ?? 0) << 8) | (c ?? 0)
    out += B64[(n >> 18) & 63]! + B64[(n >> 12) & 63]!
    out += b === undefined ? '=' : B64[(n >> 6) & 63]!
    out += c === undefined ? '=' : B64[n & 63]!
  }
  return out
}

/** The series drawn for a row, when TokenRadar has one for its window. */
export const seriesFor = (all: readonly Series[], row: Row): Series | undefined =>
  all.find(s => s.id === row.id && row.resetsAt !== undefined && Math.abs(s.resetsAt - row.resetsAt) <= 5 * 60_000)

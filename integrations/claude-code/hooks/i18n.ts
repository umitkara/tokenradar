// The mod's words in English and Turkish.
//
// The mod runs inside Claude Code, without the app's Fluent files, so it keeps
// its few strings here (ADR-043). Wording follows locales/{en,tr}/cli.ftl and
// the glossary in docs/design-language.md; numbers and durations follow
// crates/tokenradar-i18n/src/format.rs.

import type { Lang } from '../types'

export type { Lang }

type Dict = {
  limitFiveHour: string
  limitWeekly: string
  limitWeeklyScoped: (scope: string) => string
  limitSpend: string
  sourceClaudeCode: string
  sourceStatusline: string
  sourceApi: string
  sourceEstimate: string
  sourceCodexLog: string
  bandUsed: (name: string, pct: string, resets: string | null) => string
  bandProjection: (name: string, eta: string) => string
  hide: string
  toastUsed: (name: string, pct: string) => string
  toastProjection: (name: string) => string
  title: string
  colLimit: string
  colUsed: string
  colResets: string
  colSource: string
  colProjection: string
  fullIn: (dur: string) => string
  resetFirst: string
  updatedAgo: (dur: string) => string
  absent: (name: string) => string
  noLimits: string
  share: (pct: string) => string
  context: (pct: string, window: string) => string
  cost: (usd: string) => string
  dataApp: string
  dataDatabase: string
  dataNone: string
  estimateNote: string
  commandDescription: string
}

const EN: Dict = {
  limitFiveHour: '5-hour',
  limitWeekly: 'Weekly',
  limitWeeklyScoped: scope => `Weekly (${scope})`,
  limitSpend: 'Spend limit',
  sourceClaudeCode: 'Claude Code',
  sourceStatusline: 'statusline',
  sourceApi: 'API',
  sourceEstimate: 'estimate',
  sourceCodexLog: 'Codex log',
  bandUsed: (name, pct, resets) =>
    resets === null ? `${name} ${pct} used` : `${name} ${pct} used · resets in ${resets}`,
  bandProjection: (name, eta) => `${name} ≈ full in ${eta}, before the reset`,
  hide: 'Hide',
  toastUsed: (name, pct) => `TokenRadar: ${name} limit at ${pct}`,
  toastProjection: name => `TokenRadar: ${name} limit ≈ full before the reset`,
  title: 'TokenRadar · limits',
  colLimit: 'Limit',
  colUsed: 'Used',
  colResets: 'Resets in',
  colSource: 'Source',
  colProjection: 'At this rate',
  fullIn: dur => `full in ${dur}`,
  resetFirst: "won't fill",
  updatedAgo: dur => `updated ${dur} ago`,
  absent: name => `${name}: not on this account.`,
  noLimits: 'No limit readings yet.',
  share: pct => `This session's share of the 5-hour window: ≈${pct}`,
  context: (pct, window) => `Context window: ${pct} full (${window} tokens)`,
  cost: usd => `Session cost: API $${usd} (at API prices, not a bill)`,
  dataApp: 'Data: Claude Code, plus live data from the TokenRadar app.',
  dataDatabase:
    'Data: Claude Code, plus the last values TokenRadar stored (the app is not running).',
  dataNone:
    'Data: Claude Code only. TokenRadar was not found, so projections, scoped limits and the session share are missing.',
  estimateNote: '≈ marks estimates, not measurements.',
  commandDescription: 'Show usage limits, projections and their sources (TokenRadar)',
}

const TR: Dict = {
  limitFiveHour: '5 saatlik',
  limitWeekly: 'Haftalık',
  limitWeeklyScoped: scope => `Haftalık (${scope})`,
  limitSpend: 'Harcama limiti',
  sourceClaudeCode: 'Claude Code',
  sourceStatusline: 'statusline',
  sourceApi: 'API',
  sourceEstimate: 'tahmin',
  sourceCodexLog: 'Codex günlüğü',
  bandUsed: (name, pct, resets) =>
    resets === null
      ? `${name} limit: ${pct} kullanıldı`
      : `${name} limit: ${pct} kullanıldı · ${resets} içinde sıfırlanır`,
  bandProjection: (name, eta) => `${name} limit ≈ ${eta} içinde dolar, sıfırlanmadan önce`,
  hide: 'Gizle',
  toastUsed: (name, pct) => `TokenRadar: ${name} limit ${pct} seviyesinde`,
  toastProjection: name => `TokenRadar: ${name} limit sıfırlanmadan ≈ dolacak`,
  title: 'TokenRadar · limitler',
  colLimit: 'Limit',
  colUsed: 'Kullanım',
  colResets: 'Sıfırlanma',
  colSource: 'Kaynak',
  colProjection: 'Bu hızla',
  fullIn: dur => `${dur} içinde dolar`,
  resetFirst: 'dolmaz',
  updatedAgo: dur => `${dur} önce güncellendi`,
  absent: name => `${name}: bu hesapta yok.`,
  noLimits: 'Henüz limit verisi yok.',
  share: pct => `Bu oturumun 5 saatlik penceredeki payı: ≈${pct}`,
  context: (pct, window) => `Bağlam penceresi: ${pct} dolu (${window} token)`,
  cost: usd => `Oturum maliyeti: API $${usd} (API fiyatıyla, fatura değil)`,
  dataApp: 'Veri: Claude Code ve TokenRadar uygulamasından canlı veri.',
  dataDatabase:
    "Veri: Claude Code ve TokenRadar'ın son kaydettiği değerler (uygulama çalışmıyor).",
  dataNone:
    'Veri: yalnızca Claude Code. TokenRadar bulunamadı; projeksiyon, model limitleri ve oturum payı yok.',
  estimateNote: '≈ işareti ölçümü değil tahmini gösterir.',
  commandDescription: 'Kullanım limitlerini, projeksiyonları ve kaynaklarını göster (TokenRadar)',
}

export const t = (lang: Lang): Dict => (lang === 'tr' ? TR : EN)

/** `n` with `decimals` fraction digits and locale grouping: TR `1.234,5`, EN `1,234.5`. */
export function num(lang: Lang, n: number, decimals: number): string {
  const [group, point] = lang === 'tr' ? ['.', ','] : [',', '.']
  const fixed = Math.abs(n).toFixed(decimals)
  const [int = '0', frac = ''] = fixed.split('.')
  let out = n < 0 && /[1-9]/.test(fixed) ? '-' : ''
  for (let i = 0; i < int.length; i++) {
    if (i > 0 && (int.length - i) % 3 === 0) out += group
    out += int[i]
  }
  return frac === '' ? out : `${out}${point}${frac}`
}

/** At most one decimal, a trailing zero dropped: TR `%72`, `%23,5`; EN `72%`, `23.5%`. */
export function pct(lang: Lang, n: number): string {
  const rounded = Math.round(n * 10) / 10
  const digits = num(lang, rounded, Number.isInteger(rounded) ? 0 : 1)
  return lang === 'tr' ? `%${digits}` : `${digits}%`
}

/** The two largest units, truncated: `1 sa 48 dk`, `2 g 3 sa`; EN `1h 48m`, `2d 3h`. */
export function dur(lang: Lang, ms: number): string {
  const SEC = 1000
  const MIN = 60 * SEC
  const HOUR = 60 * MIN
  const DAY = 24 * HOUR
  const left = Math.max(0, ms)
  const units: Record<'d' | 'h' | 'm' | 's', string> =
    lang === 'tr' ? { d: 'g', h: 'sa', m: 'dk', s: 'sn' } : { d: 'd', h: 'h', m: 'm', s: 's' }
  const part = (n: number, u: keyof typeof units) =>
    lang === 'tr' ? `${n} ${units[u]}` : `${n}${units[u]}`
  let big: string
  let small: string | null = null
  if (left >= DAY) {
    big = part(Math.floor(left / DAY), 'd')
    const h = Math.floor((left % DAY) / HOUR)
    if (h > 0) small = part(h, 'h')
  } else if (left >= HOUR) {
    big = part(Math.floor(left / HOUR), 'h')
    const m = Math.floor((left % HOUR) / MIN)
    if (m > 0) small = part(m, 'm')
  } else if (left >= MIN) {
    big = part(Math.floor(left / MIN), 'm')
  } else {
    big = part(Math.floor(left / SEC), 's')
  }
  return small === null ? big : `${big} ${small}`
}

// The mod's `$.state` contract: what it keeps for the session, surviving a reload.
// Change a value's shape tag in register.tsx when its fields change.

export type Lang = 'en' | 'tr'

/** 0: below every threshold, 1: warn (≥ 75 % or full before the reset), 2: critical (≥ 90 %). */
export type Level = 0 | 1 | 2

/** One rate-limit window as Claude Code reported it, `resetsAt` in epoch milliseconds. */
export type Window = { kind: string; percentUsed: number; resetsAt?: number }

/** What the last `session.measure` (or `$.session.usage()`) said. */
export type Measure = {
  windows: Window[]
  contextPercent?: number
  contextWindow: number
  costUsd?: number
  at: number
}

/** One limit from `tokenradar status --json`, reduced to what the mod shows. */
export type RadarLimit = {
  /** `five_hour`, `seven_day`, `scoped:<scope>` or `other:<kind>`. */
  id: string
  percent: number
  resetsAt?: number
  /** `statusline`, `api`, `estimate`, `codex_log`. */
  source: string
  estimated: boolean
  stale: boolean
  observedAt: number
  /** TokenRadar has a projection for this window. */
  projected: boolean
  willHit: boolean
  etaAt?: number
}

/** The last answer of the TokenRadar CLI; `missing` when it could not be run or read. */
export type Radar = {
  origin: 'app' | 'database' | 'missing'
  lang: Lang
  limits: RadarLimit[]
  /** Ids of windows this account does not have (ADR-041). */
  absent: string[]
  /** This session's ≈ share of the 5-hour window. */
  share?: number
  at: number
}

/** The band is hidden up to `hidden`; a toast was raised up to `toasted`. Both reset at 0. */
export type AlertState = { hidden: Level; toasted: Level }

declare module 'claude-code' {
  interface PluginState {
    tokenradar: {
      measure: Shaped<Measure | null>
      radar: Shaped<Radar | null>
      alerts: Shaped<AlertState>
    }
  }
}

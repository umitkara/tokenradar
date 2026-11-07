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

/** A running Claude Code session from `tokenradar status --json` (ADR-046: no title). */
export type LiveSession = {
  /** The session's name, else its project folder's name. */
  label: string
  /** This very session. */
  own: boolean
  /** `busy`, `idle`, `unknown`. */
  status?: string
  /** ≈ % of the 5-hour window (an estimate). */
  share?: number
  /** API-equivalent cost in the window, USD; 0 from the database. */
  costUsd: number
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
  sessions: LiveSession[]
  at: number
}

/** One limit's stored readings in its current window, from `tokenradar history --json`. */
export type Series = {
  /** A row id, as `RadarLimit.id`. */
  id: string
  start: number
  resetsAt: number
  /** `[at, percent]`, oldest first. */
  points: [number, number][]
}

/** The last `history` answer; `series` is empty when the CLI could not give one. */
export type History = { series: Series[]; at: number }

/**
 * The reset each window was last seen heading for, and the one-off band line for a
 * window that just reset (`notice`, until hidden or an hour after the reset).
 */
export type ResetState = {
  known: Record<string, number>
  notice: { id: string; at: number } | null
}

/** The band is hidden up to `hidden`; a toast was raised up to `toasted`. Both reset at 0. */
export type AlertState = { hidden: Level; toasted: Level }

declare module 'claude-code' {
  interface PluginState {
    tokenradar: {
      measure: Shaped<Measure | null>
      radar: Shaped<Radar | null>
      alerts: Shaped<AlertState>
      history: Shaped<History | null>
      /** The pane is open: the history is kept fresh while it is. */
      pane: Shaped<boolean>
      /** Claude Code's statusline runs TokenRadar's bridge; `null` before the first look. */
      bridge: Shaped<boolean | null>
      resets: Shaped<ResetState>
    }
  }
}

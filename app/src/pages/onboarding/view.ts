// Onboarding's pure logic: the steps, how indexing and the limit sources are worded, and what
// the tray step offers for each pin state. Components only render what these return.

import type { DataSourcesView, IndexState, OsFlavor, TrayPinState } from "../../lib/ipc/bindings";
import { indexPercent } from "../../lib/progress";
import type { Message } from "../settings/view";

export type Step = "welcome" | "claude" | "limits" | "tray";

export const STEPS: readonly Step[] = ["welcome", "claude", "limits", "tray"];

export const TITLES: Record<Step, string> = {
  welcome: "onb-welcome-title",
  claude: "onb-claude-title",
  limits: "onb-limits-title",
  tray: "onb-tray-title",
};

/** The indexing ring: `pct` fills it (`null`: not started), `lines` explain it. */
export interface IndexView {
  pct: number | null;
  lines: Message[];
}

/** The first scan, from the panel snapshot's index state; `null` until the core answered. */
export function indexView(index: IndexState | null): IndexView {
  if (index?.initial_scan_done) {
    return { pct: 100, lines: [{ key: "onb-claude-indexed" }] };
  }
  if (!index?.progress) {
    return { pct: null, lines: [{ key: "onb-claude-waiting" }] };
  }
  const pct = indexPercent(index.progress);
  const lines: Message[] = [{ key: "onb-claude-indexing", args: { pct } }];
  if (index.recent_ready) lines.push({ key: "onb-claude-recent-ready" });
  return { pct, lines };
}

/** Where limit percentages will come from, for the line under the two cards. */
export type LimitsOutcome = "statusline" | "api" | "none";

export function limitsOutcome(sources: DataSourcesView | null): LimitsOutcome | null {
  if (!sources) return null;
  const s = sources.statusline.state;
  if (s === "installed" || s === "installed_elsewhere") return "statusline";
  if (sources.api.granted) return "api";
  return "none";
}

export const OUTCOME_KEYS: Record<LimitsOutcome, string> = {
  statusline: "onb-limits-ready-statusline",
  api: "onb-limits-ready-api",
  none: "onb-limits-skipped",
};

/** The tray step: a status line, whether "Always show" is offered, and the manual fallback. */
export interface PinView {
  status: Message | null;
  offerPin: boolean;
  /** Instructions plus the "open taskbar settings" button. */
  manual: Message | null;
}

/**
 * `tried`: the user already clicked "Always show", so a still hidden or unknown icon means it
 * did not work and the manual way is shown.
 */
export function pinView(state: TrayPinState | null, os: OsFlavor, tried: boolean): PinView {
  const manualKey = os === "win10" ? "onb-tray-manual-win10" : "onb-tray-manual";
  switch (state) {
    case "shown":
      return { status: { key: "onb-tray-shown" }, offerPin: false, manual: null };
    case "unsupported":
      return { status: null, offerPin: false, manual: { key: manualKey } };
    case "hidden":
    case "unknown":
      return tried
        ? { status: { key: "onb-tray-not-done" }, offerPin: false, manual: { key: manualKey } }
        : { status: null, offerPin: true, manual: null };
    default:
      return { status: null, offerPin: false, manual: null };
  }
}

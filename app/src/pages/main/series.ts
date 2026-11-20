// The overview's time series (`get_overview().series`) as a chart model. Field mapping and the
// color role only: values, thinning and window flags come from the backend.

import type { ChartBand, ChartModel, Tone } from "../../lib/chart/model";
import { limitName } from "../../lib/components/limit";
import type { TimeSeries } from "../../lib/ipc/bindings";

/** Color follows the limit, never its rank: weekly keeps its tone on a single-limit account. */
export function toneOf(series: TimeSeries): Tone {
  return series.limit.kind === "session" ? "primary" : "secondary";
}

/**
 * Lines for the 5-hour window and the weekly limit, in that order. Window bands come from the
 * shortest-window line shown (5-hour if present); hit-limit and estimated windows from every line.
 */
export function chartOf(series: readonly TimeSeries[], from: number, to: number): ChartModel {
  const shown = series
    .filter((s) => s.limit.kind === "session" || s.limit.kind === "weekly_all")
    .sort((a, b) => Number(b.limit.kind === "session") - Number(a.limit.kind === "session"));
  const bands: ChartBand[] = [];
  const first = shown[0];
  for (const w of first?.windows ?? []) {
    bands.push({ from: w.starts_at, to: w.resets_at, kind: "window" });
  }
  for (const s of shown) {
    for (const w of s.windows) {
      if (w.estimated) bands.push({ from: w.starts_at, to: w.resets_at, kind: "estimated" });
      else if (w.hit_limit === true)
        bands.push({ from: w.starts_at, to: w.resets_at, kind: "hit" });
    }
  }
  return {
    from,
    to,
    yMax: 100,
    thresholds: [75, 90],
    bands,
    lines: shown.map((s) => ({
      id: s.limit.kind === "session" ? "five_hour" : "weekly",
      label: limitName(s.limit),
      tone: toneOf(s),
      points: s.points.flatMap((p) =>
        p.percent == null
          ? []
          : [{ at: p.at, value: p.percent, estimated: p.source === "estimate" }],
      ),
    })),
  };
}

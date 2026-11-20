// Chart fixtures for the gallery (Step 15): synthetic, relative to the page's load time.

import type { ChartLine, ChartModel, ChartSpan } from "../../lib/chart/model";

const HOUR = 3_600_000;
const DAY = 24 * HOUR;

/** A 7-day range ending now with a 5-hour and a weekly line. */
function range(): { from: number; to: number } {
  const to = Date.now();
  return { from: to - 7 * DAY, to };
}

function weeklyLine(from: number, to: number, label: string, estimateTail: boolean): ChartLine {
  const points = [];
  let value = 0;
  for (let at = from; at <= to; at += 2 * HOUR) {
    // Resets after 3 days; daytime use only.
    if (at - from > 3 * DAY && at - from <= 3 * DAY + 2 * HOUR) value = 0;
    const hour = new Date(at).getHours();
    if (hour >= 9 && hour < 23) value += 1.6;
    points.push({
      at,
      value: Math.min(100, value),
      estimated: estimateTail && to - at < 10 * HOUR,
    });
  }
  return { id: "weekly", label, tone: "secondary", points };
}

function fiveHourLine(from: number, to: number, label: string): ChartLine {
  const points = [];
  for (let at = from; at <= to; at += HOUR) {
    const inWindow = (at - from) % (5 * HOUR);
    const hour = new Date(at).getHours();
    const busy = hour >= 9 && hour < 23;
    points.push({ at, value: busy ? Math.min(100, (inWindow / HOUR) * 22) : 0 });
  }
  return { id: "five_hour", label, tone: "primary", points };
}

export function twoLines(names: { fiveHour: string; weekly: string }): ChartModel {
  const { from, to } = range();
  const bands: ChartModel["bands"] = [];
  for (let at = from; at < to; at += 5 * HOUR)
    bands.push({ from: at, to: at + 5 * HOUR, kind: "window" });
  bands.push({ from: from + 2 * DAY, to: from + 2 * DAY + 5 * HOUR, kind: "hit" });
  return {
    from,
    to,
    yMax: 100,
    thresholds: [75, 90],
    bands,
    lines: [fiveHourLine(from, to, names.fiveHour), weeklyLine(from, to, names.weekly, true)],
    spans: spans(from),
  };
}

/** Five sessions for the lane under the chart: one long, one too short to see, one without a share. */
function spans(from: number): ChartSpan[] {
  const at = (day: number, hour: number) => from + day * DAY + hour * HOUR;
  return [
    { id: "a", from: at(0, 10), to: at(0, 18), label: "refactor-indexer", percent: 8.4 },
    { id: "b", from: at(2, 9), to: at(2, 14), label: "fix-tray-icon", percent: 5.1 },
    { id: "c", from: at(3, 20), to: at(3, 20.2), label: "quick question", percent: 0.6 },
    { id: "d", from: at(4, 11), to: at(5, 2), label: "migrate-db", percent: 3.2 },
    { id: "e", from: at(6, 8), to: at(6, 12), label: "docs pass", percent: null },
  ];
}

/** Daily cost series for the sparkline demos: rising, flat at zero, one busy day. */
export const sparks: { key: string; values: number[] }[] = [
  { key: "gallery-spark-rising", values: [0.2, 0.4, 0.3, 0.9, 1.4, 1.2, 2.1] },
  { key: "gallery-spark-flat", values: [0, 0, 0, 0, 0, 0, 0] },
  { key: "gallery-spark-spike", values: [0.1, 0.1, 3.5, 0.2, 0.1, 0, 0.1] },
];

export function weeklyOnly(name: string): ChartModel {
  const { from, to } = range();
  return {
    from,
    to,
    yMax: 100,
    thresholds: [75, 90],
    bands: [
      { from: from - 4 * DAY, to: from + 3 * DAY, kind: "window" },
      { from: from + 3 * DAY, to: from + 10 * DAY, kind: "window" },
    ],
    lines: [weeklyLine(from, to, name, false)],
  };
}

export function emptyChart(): ChartModel {
  const { from, to } = range();
  return { from, to, yMax: 100, thresholds: [75, 90], bands: [], lines: [] };
}

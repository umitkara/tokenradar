// Chart fixtures for the gallery (Step 15): synthetic, relative to the page's load time.

import type { ChartLine, ChartModel } from "../../lib/chart/model";

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
  };
}

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

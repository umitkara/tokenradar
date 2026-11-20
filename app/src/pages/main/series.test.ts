import { describe, expect, it } from "vitest";
import type { TimeSeries } from "../../lib/ipc/bindings";
import { chartOf } from "./series";

const series = (
  kind: "session" | "weekly_all",
  windows: TimeSeries["windows"] = [],
): TimeSeries => ({
  limit: { kind, scope: null },
  range: { from: 0, to: 100 },
  points: [
    { at: 10, percent: 5, source: "statusline" },
    { at: 20, percent: null, source: "api" },
    { at: 30, percent: 7, source: "api" },
  ],
  windows,
  markers: [],
});

const band = (id: number, from: number, to: number, hit: boolean | null, estimated = false) => ({
  window_id: id,
  starts_at: from,
  resets_at: to,
  peak_percent: null,
  hit_limit: hit,
  estimated,
});

describe("chartOf", () => {
  it("orders 5-hour before weekly and keeps each limit's tone", () => {
    const m = chartOf([series("weekly_all"), series("session")], 0, 100);
    expect(m.lines.map((l) => [l.id, l.tone])).toEqual([
      ["five_hour", "primary"],
      ["weekly", "secondary"],
    ]);
  });

  it("keeps weekly's tone when it is the only line", () => {
    expect(chartOf([series("weekly_all")], 0, 100).lines.map((l) => l.tone)).toEqual(["secondary"]);
  });

  it("drops points without a value", () => {
    expect(chartOf([series("session")], 0, 100).lines[0]?.points.map((p) => p.at)).toEqual([
      10, 30,
    ]);
  });

  it("draws window bands from the first line and hit or estimated windows from all", () => {
    const m = chartOf(
      [
        series("session", [band(1, 0, 50, false), band(2, 50, 100, true)]),
        series("weekly_all", [band(3, -10, 90, null, true)]),
      ],
      0,
      100,
    );
    expect(m.bands).toEqual([
      { from: 0, to: 50, kind: "window" },
      { from: 50, to: 100, kind: "window" },
      { from: 50, to: 100, kind: "hit" },
      { from: -10, to: 90, kind: "estimated" },
    ]);
    expect(m.thresholds).toEqual([75, 90]);
  });
});

import { describe, expect, it } from "vitest";
import {
  align,
  type ChartLine,
  latest,
  shortDate,
  tickLabel,
  timeTicks,
  tooltipTime,
  valueAt,
} from "./model";

const line = (id: string, points: [number, number, boolean?][]): ChartLine => ({
  id,
  label: id,
  tone: "primary",
  points: points.map(([at, value, estimated]) => ({ at, value, estimated })),
});

// Local times, so the tests pass in any time zone.
const local = (d: number, h = 0, m = 0) => new Date(2026, 10, d, h, m).getTime();
const DAY = 86_400_000;

describe("align", () => {
  it("merges x values; another line's x is undefined (joined)", () => {
    const a = align([
      line("a", [
        [1, 10],
        [3, 30],
      ]),
      line("b", [
        [2, 20],
        [3, 31],
      ]),
    ]);
    expect(a.xs).toEqual([1, 2, 3]);
    expect(a.series).toEqual([
      { lineId: "a", tone: "primary", dashed: false, values: [10, undefined, 30] },
      { lineId: "b", tone: "primary", dashed: false, values: [undefined, 20, 31] },
    ]);
  });

  it("splits estimated runs into a dashed series joined at the boundary, broken elsewhere", () => {
    const a = align([
      line("a", [
        [1, 10],
        [2, 20],
        [3, 25, true],
        [4, 30, true],
        [5, 32],
      ]),
    ]);
    expect(a.series).toEqual([
      { lineId: "a", tone: "primary", dashed: false, values: [10, 20, null, 30, 32] },
      { lineId: "a", tone: "primary", dashed: true, values: [null, 20, 25, 30, null] },
    ]);
  });

  it("emits only the kinds a line has", () => {
    expect(align([line("a", [[1, 1, true]])]).series.map((s) => s.dashed)).toEqual([true]);
    expect(align([line("a", [[1, 1]])]).series.map((s) => s.dashed)).toEqual([false]);
    expect(align([line("a", [])]).series).toEqual([]);
    expect(align([]).xs).toEqual([]);
  });
});

describe("valueAt and latest", () => {
  const l = line("a", [
    [10, 1],
    [20, 2],
    [30, 3],
  ]);
  it("finds the point at or before a time", () => {
    expect(valueAt(l, 5)).toBeNull();
    expect(valueAt(l, 10)?.value).toBe(1);
    expect(valueAt(l, 25)?.value).toBe(2);
    expect(valueAt(l, 99)?.value).toBe(3);
  });
  it("returns the last point", () => {
    expect(latest(l)?.value).toBe(3);
    expect(latest(line("e", []))).toBeNull();
  });
});

describe("time ticks", () => {
  it("puts day ticks on local midnights for long ranges", () => {
    const from = local(13, 11, 0);
    const ticks = timeTicks(from, from + 7 * DAY);
    expect(ticks).toHaveLength(7);
    expect(ticks[0]).toBe(local(14));
    for (const t of ticks) {
      const d = new Date(t);
      expect([d.getHours(), d.getMinutes()]).toEqual([0, 0]);
    }
  });

  it("puts hour ticks on multiples of 3 or 6 hours for short ranges", () => {
    const from = local(20, 10, 20);
    expect(timeTicks(from, from + 5 * 3_600_000).map((t) => new Date(t).getHours())).toEqual([
      12, 15,
    ]);
    expect(timeTicks(from, from + DAY).map((t) => new Date(t).getHours())).toEqual([12, 18, 0, 6]);
  });
});

describe("labels in both languages", () => {
  it("formats dates and ticks", () => {
    expect(shortDate("tr", local(20))).toBe("20 Kas");
    expect(shortDate("en", local(20))).toBe("Nov 20");
    expect(tickLabel("tr", false, local(20), 7 * DAY)).toBe("Cum 20");
    expect(tickLabel("en", false, local(20), 7 * DAY)).toBe("Fri 20");
    expect(tickLabel("en", true, local(20, 15), DAY)).toBe("3:00 PM");
    expect(tickLabel("tr", true, local(20, 15), DAY)).toBe("15:00");
  });

  it("formats the tooltip time", () => {
    expect(tooltipTime("tr", false, local(19, 14, 5))).toBe("Per 19 Kas 14:05");
    expect(tooltipTime("en", true, local(19, 14, 5))).toBe("Thu Nov 19 2:05 PM");
  });
});

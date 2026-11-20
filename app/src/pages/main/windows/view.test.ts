import { describe, expect, it } from "vitest";
import type { CycleRow, MixTrend } from "../../../lib/ipc/bindings";
import {
  barHeight,
  blockBox,
  contribSeries,
  dayMax,
  familyColor,
  firstWeekday,
  heatStep,
  mixShares,
  nearestIndex,
  stackMax,
  stackPaths,
  toJsDay,
  weekOrder,
} from "./view";

function row(days: { percent: number | null; cost: number }[]): CycleRow {
  return {
    window: {} as CycleRow["window"],
    days: days.map((d, i) => ({ starts_at: i, percent: d.percent, cost: d.cost, future: false })),
    hit_at: null,
    hit_day: null,
    projects: [],
    breakdown: [],
    scoped: [],
  };
}

describe("day bars", () => {
  it("scales by percent when any day has one", () => {
    const rows = [row([{ percent: 4, cost: 900 }]), row([{ percent: 10, cost: 100 }])];
    expect(dayMax(rows)).toEqual({ max: 10, byCost: false });
  });

  it("falls back to cost without a calibration", () => {
    expect(
      dayMax([
        row([
          { percent: null, cost: 900 },
          { percent: null, cost: 300 },
        ]),
      ]),
    ).toEqual({
      max: 900,
      byCost: true,
    });
  });

  it("clamps heights and leaves empty days empty", () => {
    expect(barHeight(5, 10)).toBe(50);
    expect(barHeight(0, 10)).toBe(0);
    expect(barHeight(12, 10)).toBe(100);
    expect(barHeight(3, 0)).toBe(0);
  });
});

describe("week order", () => {
  it("uses the table when the system language differs", () => {
    expect(firstWeekday("tr", "en-US")).toBe(0);
  });

  it("returns a Monday-zero day", () => {
    const d = firstWeekday("en", "en-US");
    expect(d).toBeGreaterThanOrEqual(0);
    expect(d).toBeLessThan(7);
  });

  it("orders from the first day and maps to Date#getDay", () => {
    expect(weekOrder(6)).toEqual([6, 0, 1, 2, 3, 4, 5]);
    expect(toJsDay(0)).toBe(1);
    expect(toJsDay(6)).toBe(0);
  });
});

describe("heat steps", () => {
  it("gives any usage at least the first step", () => {
    expect(heatStep(0, 100)).toBe(0);
    expect(heatStep(1, 100)).toBe(1);
    expect(heatStep(100, 100)).toBe(5);
    expect(heatStep(41, 100)).toBe(3);
  });
});

describe("stacks", () => {
  it("stacks series bottom up", () => {
    const paths = stackPaths(
      [
        [1, 1],
        [1, 3],
      ],
      100,
      40,
      4,
    );
    expect(paths[0]).toBe("M0.0,30.0L100.0,30.0L100.0,40.0L0.0,40.0Z");
    expect(paths[1]).toBe("M0.0,20.0L100.0,0.0L100.0,30.0L0.0,30.0Z");
    expect(
      stackMax([
        [1, 1],
        [1, 3],
      ]),
    ).toBe(4);
  });

  it("draws nothing without data", () => {
    expect(stackPaths([[]], 100, 40, 4)).toEqual([""]);
    expect(stackPaths([[1]], 100, 40, 0)).toEqual([""]);
  });

  it("carries running totals over gaps", () => {
    expect(contribSeries([[null, 2, null, 5]])).toEqual([[0, 2, 2, 5]]);
  });

  it("splits weeks into shares", () => {
    const trend: MixTrend = {
      families: ["opus", null],
      weeks: [
        { starts_at: 0, total: 400, parts: [300, 100] },
        { starts_at: 1, total: 0, parts: [0, 0] },
      ],
    };
    expect(mixShares(trend)).toEqual([
      [75, 0],
      [25, 0],
    ]);
  });
});

describe("placement", () => {
  it("places strip blocks on the day", () => {
    expect(blockBox(360, 660, 1440)).toEqual({ left: 25, width: 20.833333333333336 });
    expect(blockBox(0, 1500, 1500).width).toBe(100);
  });

  it("picks the nearest point", () => {
    expect(nearestIndex(0.49, 3)).toBe(1);
    expect(nearestIndex(-1, 3)).toBe(0);
    expect(nearestIndex(2, 3)).toBe(2);
    expect(nearestIndex(0.5, 1)).toBe(0);
  });

  it("colours by family, not rank", () => {
    expect(familyColor("opus")).toBe("var(--series-2)");
    expect(familyColor("haiku")).toBe("var(--series-5)");
    expect(familyColor(null)).toBe("var(--text-3)");
  });
});

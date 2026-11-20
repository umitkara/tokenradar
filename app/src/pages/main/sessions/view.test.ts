import { describe, expect, it } from "vitest";
import type { Basis, ImpactRow, ModelFamily } from "../../../lib/ipc/bindings";
import {
  costAxis,
  costChart,
  costTicks,
  durationOf,
  impactOf,
  impactRows,
  middleEllipsis,
  modelLabel,
  stackMax,
  stackOf,
  windowBands,
} from "./view";

const basis = (over: Partial<Basis> = {}): Basis => ({
  method: "proportional",
  confidence: null,
  approx: true,
  dim: true,
  stale: false,
  ...over,
});

const impact = (over: Partial<ImpactRow>): ImpactRow => ({
  window_id: 1,
  kind: "weekly_all",
  scope: null,
  starts_at: 0,
  resets_at: 100,
  peak_percent: 50,
  hit_limit: false,
  estimated: false,
  percent: 3,
  cost: 1_000_000,
  basis: basis(),
  ...over,
});

describe("middleEllipsis", () => {
  it("keeps short names and cuts long ones in the middle", () => {
    expect(middleEllipsis("short", 10)).toBe("short");
    const cut = middleEllipsis("abcdefghijklmnopqrstuvwxyz", 11);
    expect(cut).toBe("abcde…vwxyz");
    expect([...cut]).toHaveLength(11);
  });

  it("does not split surrogate pairs", () => {
    expect(middleEllipsis("😀😀😀😀😀😀😀😀", 5)).toBe("😀😀…😀😀");
  });
});

describe("modelLabel", () => {
  it("shortens model ids", () => {
    expect(modelLabel("claude-opus-5-5")).toBe("Opus 5.5");
    expect(modelLabel("claude-haiku-4-5-20251001")).toBe("Haiku 4.5");
    expect(modelLabel("claude-sonnet-4-20250514")).toBe("Sonnet 4");
    expect(modelLabel("claude-fable-5-2")).toBe("Fable 5.2");
    expect(modelLabel("<synthetic>")).toBe("<synthetic>");
    expect(modelLabel("claude-3-5-sonnet-latest")).toBe("3-5-sonnet-latest");
  });
});

describe("impactOf", () => {
  it("prefers the weekly share and falls back to the 5-hour one", () => {
    const b = basis({ dim: false });
    expect(impactOf({ weekly_percent: 4, session_percent: 9, impact_basis: b })).toEqual({
      kind: "weekly",
      percent: 4,
      dim: false,
    });
    expect(impactOf({ weekly_percent: null, session_percent: 9, impact_basis: null })).toEqual({
      kind: "session",
      percent: 9,
      dim: true,
    });
    expect(
      impactOf({ weekly_percent: null, session_percent: null, impact_basis: null }),
    ).toBeNull();
  });
});

describe("durationOf", () => {
  it("needs both ends", () => {
    expect(durationOf({ first_ts: 10, last_ts: 70 })).toBe(60);
    expect(durationOf({ first_ts: null, last_ts: 70 })).toBeNull();
  });
});

describe("costChart", () => {
  const b = (at: number, model: string, family: ModelFamily, cost: number) => ({
    at,
    model,
    family,
    cost,
  });

  it("stacks families in a fixed order on shared buckets", () => {
    const c = costChart(
      [
        b(0, "claude-sonnet-5-5", "sonnet", 9),
        b(0, "claude-opus-5-5", "opus", 1),
        b(300, "claude-opus-4-6", "opus", 2),
        b(300, "claude-opus-5-5", "opus", 3),
      ],
      300_000,
    );
    expect(c.xs).toEqual([0, 300]);
    expect(c.stacks).toEqual([
      {
        key: "opus",
        token: "--series-2",
        models: ["claude-opus-5-5", "claude-opus-4-6"],
        values: [1, 5],
      },
      { key: "sonnet", token: "--series-4", models: ["claude-sonnet-5-5"], values: [9, 0] },
    ]);
  });

  it("folds other families together and is empty without buckets", () => {
    const c = costChart([b(0, "x", "mythos", 2), b(0, "<synthetic>", "synthetic", 1)], 300_000);
    expect(c.stacks.map((s) => [s.key, s.values])).toEqual([["other", [3]]]);
    expect(costChart([], 300_000)).toEqual({ xs: [], stacks: [], bucketMs: 300_000 });
  });

  it("maps families to stacks", () => {
    expect(stackOf("haiku")).toBe("haiku");
    expect(stackOf("unknown")).toBe("other");
  });
});

describe("windowBands and impactRows", () => {
  const rows = [
    impact({ window_id: 1, kind: "weekly_all", starts_at: 0, resets_at: 700 }),
    impact({ window_id: 2, kind: "session", starts_at: 100, resets_at: 200, hit_limit: true }),
    impact({ window_id: 3, kind: "session", starts_at: 300, resets_at: 400 }),
    impact({ window_id: 4, kind: "weekly_scoped", scope: { model: "Fable" }, resets_at: 700 }),
  ];

  it("draws 5-hour windows when there are any, weekly ones otherwise", () => {
    expect(windowBands(rows)).toEqual([
      { from: 100, to: 200, kind: "hit" },
      { from: 300, to: 400, kind: "window" },
    ]);
    expect(windowBands([impact({ estimated: true, hit_limit: null })])).toEqual([
      { from: 0, to: 100, kind: "estimated" },
    ]);
  });

  it("orders 5-hour, weekly, scoped; newest first", () => {
    expect(impactRows(rows).map((r) => r.window_id)).toEqual([3, 2, 1, 4]);
  });
});

describe("cost axis and ticks", () => {
  it("rounds the axis top to 1, 2 or 5 steps", () => {
    expect(costAxis(0)).toEqual({ top: 4, step: 1 });
    expect(costAxis(3.2)).toEqual({ top: 4, step: 1 });
    expect(costAxis(4.1)).toEqual({ top: 8, step: 2 });
    expect(costAxis(13)).toEqual({ top: 20, step: 5 });
    expect(costAxis(950_000)).toEqual({ top: 2_000_000, step: 500_000 });
  });

  it("takes the highest stacked bar", () => {
    const chart = {
      xs: [0, 1],
      bucketMs: 1,
      stacks: [
        { key: "opus" as const, token: "", models: [], values: [1, 4] },
        { key: "other" as const, token: "", models: [], values: [2, 0] },
      ],
    };
    expect(stackMax(chart)).toBe(4);
  });

  it("labels short spans every few minutes and long ones by the hour", () => {
    const at = (h: number, m: number) => new Date(2026, 10, 20, h, m).getTime();
    expect(costTicks(at(10, 3), at(10, 40))).toEqual([
      at(10, 10),
      at(10, 20),
      at(10, 30),
      at(10, 40),
    ]);
    expect(costTicks(at(9, 0), at(11, 0))).toEqual([
      at(9, 0),
      at(9, 30),
      at(10, 0),
      at(10, 30),
      at(11, 0),
    ]);
    expect(costTicks(at(0, 0), at(12, 0))).toEqual([
      at(0, 0),
      at(3, 0),
      at(6, 0),
      at(9, 0),
      at(12, 0),
    ]);
  });
});

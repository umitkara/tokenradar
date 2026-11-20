import { describe, expect, it } from "vitest";
import type { Overview, SessionMarker, TimeSeries } from "../../../lib/ipc/bindings";
import {
  changeOf,
  countOf,
  dataState,
  modelRows,
  sharesSettled,
  spansOf,
  surfaceKey,
  tokenRows,
} from "./view";

function marker(id: number, percent: number | null): SessionMarker {
  return {
    session_id: id,
    name: { text: `s${id}`, fallback: false },
    start: id * 1000,
    end: id * 1000 + 500,
    percent,
  };
}

function series(kind: "session" | "weekly_all", markers: SessionMarker[]): TimeSeries {
  return {
    limit: { kind, scope: null },
    range: { from: 0, to: 1 },
    points: [],
    windows: [],
    markers,
  };
}

describe("tokenRows", () => {
  it("keeps a fixed order and sizes against the largest kind", () => {
    const rows = tokenRows({
      tokens: { input: 100, output: 50, cache_read: 800, cache_write: 50 },
      cache_write_5m: 50,
      cache_write_1h: 0,
    });
    expect(rows.map((r) => r.key)).toEqual(["input", "output", "cache_read", "cache_write"]);
    expect(rows[2]).toMatchObject({ width: 100, share: 80 });
    expect(rows[0]).toMatchObject({ width: 12.5, share: 10 });
  });

  it("draws nothing for an empty week", () => {
    const rows = tokenRows({
      tokens: { input: 0, output: 0, cache_read: 0, cache_write: 0 },
      cache_write_5m: 0,
      cache_write_1h: 0,
    });
    expect(rows.every((r) => r.width === 0 && r.share === 0)).toBe(true);
  });
});

describe("modelRows", () => {
  it("folds models past the limit into one row", () => {
    const models = [5, 4, 3, 2].map((c, i) => ({
      model: `m${i}`,
      family: "opus" as const,
      cost: c,
      tokens: 0,
    }));
    const rows = modelRows(models, 2);
    expect(rows.map((r) => [r.key, r.value])).toEqual([
      ["m0", 5],
      ["m1", 4],
      ["", 5],
    ]);
    expect(rows[0]?.width).toBe(100);
  });
});

describe("spansOf", () => {
  it("takes the weekly limit's largest shares, then the backend's cost order", () => {
    const weekly = series("weekly_all", [
      marker(1, 2),
      marker(2, null),
      marker(3, 9),
      marker(4, 2),
      marker(5, 1),
      marker(6, 0.5),
    ]);
    const five = series("session", [marker(9, 50)]);
    const spans = spansOf([five, weekly], 5);
    expect(spans.map((s) => s.id)).toEqual(["3", "1", "4", "5", "6"]);
    expect(spans[0]).toMatchObject({ from: 3000, to: 3500, label: "s3", percent: 9 });
  });

  it("falls back to the 5-hour window, and to nothing", () => {
    expect(spansOf([series("session", [marker(1, 3)])]).map((s) => s.id)).toEqual(["1"]);
    expect(spansOf([])).toEqual([]);
  });
});

describe("changeOf", () => {
  it("gives a direction and whole percent, flat under half a percent", () => {
    expect(changeOf(0.12)).toEqual({ dir: "up", pct: 12 });
    expect(changeOf(-0.5)).toEqual({ dir: "down", pct: 50 });
    expect(changeOf(0.004)).toEqual({ dir: "flat", pct: 0 });
    expect(changeOf(null)).toBeNull();
  });
});

describe("countOf", () => {
  it("shortens from ten thousand up", () => {
    expect(countOf(9_999)).toEqual({ unit: "one", n: 9_999 });
    expect(countOf(12_400)).toEqual({ unit: "k", n: 12.4 });
    expect(countOf(3_100_000)).toEqual({ unit: "m", n: 3.1 });
    expect(countOf(1_200_000_000)).toEqual({ unit: "b", n: 1.2 });
  });
});

describe("surfaceKey", () => {
  it("names the known breakdown rows and leaves others as they are", () => {
    expect(surfaceKey("chat")).toBe("overview-surface-chat");
    expect(surfaceKey("oauth_apps")).toBeNull();
  });
});

describe("dataState", () => {
  const base = {
    limits: [],
    basis: null,
    calibration: null,
  } as unknown as Overview;

  it("knows when nothing was read yet", () => {
    expect(dataState(base)).toEqual({ noLimits: true, estimatesOnly: false, learning: "none" });
  });

  it("flags a page of estimates only, and settled shares", () => {
    const o = {
      ...base,
      limits: [{ source: "estimate" }],
      basis: { method: "calibrated", confidence: 0.8, approx: false, dim: false, stale: false },
    } as unknown as Overview;
    expect(dataState(o)).toEqual({ noLimits: false, estimatesOnly: true, learning: null });
  });

  it("is still learning while the calibration is low", () => {
    const cal = { confidence: 0.2, level: "low", n_obs: 14, drift: null };
    const o = {
      ...base,
      limits: [{ source: "api" }],
      basis: { method: "proportional", confidence: 0.2, approx: true, dim: true, stale: false },
      calibration: cal,
    } as unknown as Overview;
    expect(dataState(o).learning).toEqual(cal);
    expect(sharesSettled(o.basis)).toBe(false);
  });
});

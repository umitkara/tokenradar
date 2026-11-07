import { render } from "svelte/server";
import { describe, expect, it } from "vitest";
import { timeMark } from "../../lib/components/limit";
import { initI18n } from "../../lib/i18n/i18n.svelte";
import { limit, weekly } from "../gallery/fixtures";
import { panelCatalog } from "./catalog";
import Detail from "./Detail.svelte";
import {
  CHART,
  chartY,
  clearOfNow,
  gridlines,
  localHours,
  localMidnights,
  position,
  sessionBars,
  stripDays,
  usagePaths,
} from "./detail";
import { detailOf, session, snapshot } from "./fixtures";

const HOUR = 3_600_000;
const DAY = 24 * HOUR;

/** A local wall-clock instant, in the time zone the tests run in. */
const local = (y: number, m: number, d: number, h = 0, min = 0) =>
  new Date(y, m - 1, d, h, min).getTime();

describe("position", () => {
  it("maps an instant into the window, clamped", () => {
    expect(position(50, 0, 100)).toBe(50);
    expect(position(-5, 0, 100)).toBe(0);
    expect(position(150, 0, 100)).toBe(100);
    expect(position(5, 10, 10)).toBe(0);
  });
});

describe("localMidnights", () => {
  it("cuts a week from Wednesday 11:00 at each local midnight in between", () => {
    const start = local(2026, 11, 4, 11);
    const cuts = localMidnights(start, start + 7 * DAY);
    expect(cuts).toHaveLength(7);
    for (const at of cuts) {
      const d = new Date(at);
      expect([d.getHours(), d.getMinutes()]).toEqual([0, 0]);
    }
    expect(new Date(cuts[0] ?? 0).getDate()).toBe(5);
  });

  it("leaves out a midnight the window starts or ends on", () => {
    const start = local(2026, 11, 4);
    const end = local(2026, 11, 6);
    expect(localMidnights(start, end)).toEqual([local(2026, 11, 5)]);
  });

  it("stays on local midnight across a daylight-saving change", () => {
    // Europe and North America change their clocks in late October / early November; in a
    // zone without daylight saving every day is 24 hours and the check still holds.
    const start = local(2026, 10, 22, 11);
    for (const at of localMidnights(start, local(2026, 11, 8, 11))) {
      expect(new Date(at).getHours()).toBe(0);
    }
  });
});

describe("localHours and gridlines", () => {
  it("gives whole hours for the 5-hour window and midnights for a week", () => {
    const start = local(2026, 11, 4, 9, 20);
    expect(localHours(start, start + 5 * HOUR).map((at) => new Date(at).getHours())).toEqual([
      10, 11, 12, 13, 14,
    ]);
    expect(gridlines(start, start + 5 * HOUR)).toHaveLength(5);
    expect(gridlines(start, start + 7 * DAY)).toEqual(localMidnights(start, start + 7 * DAY));
  });
});

describe("stripDays", () => {
  it("names each day that has room, and not a sliver of one", () => {
    const early = stripDays(local(2026, 11, 4, 1), local(2026, 11, 11, 1));
    expect(early).toHaveLength(8);
    expect(early.slice(0, 7).every((d) => d.label)).toBe(true);
    // The last day is the hour before the reset: no room.
    expect(early.at(-1)?.label).toBe(false);
    // Starts at 22:00: two hours of the first day, no room for a name.
    const late = stripDays(local(2026, 11, 4, 22), local(2026, 11, 11, 22));
    expect(late[0]?.label).toBe(false);
    expect(late[0]?.to).toBeCloseTo((2 / 168) * 100, 0);
    expect(late.at(-1)?.to).toBe(100);
  });
});

describe("clearOfNow", () => {
  it("keeps the 24-hour values away from the now mark", () => {
    expect(clearOfNow(42.9, 45)).toBe(false);
    expect(clearOfNow(28.6, 45)).toBe(true);
  });

  it("leaves more room on the side the word runs to", () => {
    // "▲ now" at 57 % runs right: 71 % would touch it, 43 % is clear.
    expect(clearOfNow(71.4, 57)).toBe(false);
    expect(clearOfNow(42.9, 57)).toBe(true);
    // Near the right edge the word runs left.
    expect(clearOfNow(71.4, 85, true)).toBe(false);
    expect(clearOfNow(100, 85, true)).toBe(true);
  });
});

describe("usagePaths", () => {
  it("draws the measured points and closes the area on the baseline", () => {
    const { line, area } = usagePaths(
      [
        { at: 0, percent: 0, source: "statusline" },
        { at: 50, percent: 50, source: "api" },
      ],
      0,
      100,
    );
    expect(line).toBe(
      `M0.0,${chartY(0).toFixed(1)} L${(CHART.width / 2).toFixed(1)},${chartY(50).toFixed(1)}`,
    );
    expect(area.endsWith("Z")).toBe(true);
  });

  it("is empty without points", () => {
    expect(usagePaths([], 0, 100)).toEqual({ line: "", area: "" });
  });
});

describe("sessionBars", () => {
  it("draws ≈ shares when the core knows them", () => {
    const bars = sessionBars([
      session(1, { share_percent: 12 }),
      session(2, { share_percent: null }),
    ]);
    expect(bars).toEqual([{ width: 12, estimate: true }, null]);
  });

  it("falls back to the part of the cost with two costed sessions, else draws nothing", () => {
    const shareless = { share_percent: null };
    const two = sessionBars([
      session(1, { ...shareless, cost: 3_000_000 }),
      session(2, { ...shareless, cost: 1_000_000 }),
    ]);
    expect(two).toEqual([
      { width: 75, estimate: false },
      { width: 25, estimate: false },
    ]);
    expect(sessionBars([session(1, { ...shareless, cost: 3_000_000 })])).toEqual([null]);
  });
});

describe("timeMark", () => {
  it("places the time gone on the bar, and nothing outside the window", () => {
    const l = weekly(30, { resetsIn: 3.5 * DAY });
    const now = Date.now();
    expect(timeMark(l, now)).toBeCloseTo(50, 0);
    expect(timeMark({ ...l, window_start: null }, now)).toBeNull();
    expect(timeMark(l, (l.resets_at ?? 0) + 1)).toBeNull();
  });
});

describe("Detail", () => {
  it("shows the pace, the time strip, the chart and the session costs (TR)", () => {
    initI18n(panelCatalog, "tr");
    const snap = snapshot({
      limits: [limit(42), weekly(64, { resetsIn: 3 * DAY })],
      live_sessions: [session(1, { cost: 2_570_000 }), session(2, { cost: 1_000_000 })],
    });
    const { body } = render(Detail, { props: { snapshot: snap, detail: detailOf(snap) } });
    expect(body).toContain("Geçen zaman");
    expect(body).toContain("puan önünde");
    expect(body).toContain("Haftalık limit · geçen zaman");
    expect(body).toContain("şimdi");
    expect(body).toContain("<svg");
    expect(body).toContain("API $2,57");
    expect(body).toContain("gece yarısı");
  });

  it("says when the window has no stored readings yet (EN)", () => {
    initI18n(panelCatalog, "en");
    const snap = snapshot({ limits: [weekly(10)] });
    const { body } = render(Detail, { props: { snapshot: snap, detail: detailOf(snap, false) } });
    expect(body).toContain("No stored readings of this window yet.");
    expect(body).toContain("behind the time line");
  });

  it("shows a placeholder until the detail arrives", () => {
    initI18n(panelCatalog, "en");
    const { body } = render(Detail, { props: { snapshot: snapshot(), detail: null } });
    expect(body).toContain("Loading details");
  });
});

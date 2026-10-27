import { describe, expect, it } from "vitest";
import { Ticker, type Timers, type Visibility } from "./ticker";

/** Manual time and visibility. */
function harness(start = 10_250) {
  let time = start;
  let hidden = false;
  const pending = new Map<number, { at: number; cb: () => void }>();
  const visibilityListeners = new Set<() => void>();
  let next = 1;
  const timers: Timers = {
    now: () => time,
    setTimeout: (cb, ms) => {
      pending.set(next, { at: time + ms, cb });
      return next++;
    },
    clearTimeout: (h) => pending.delete(h as number),
  };
  const visibility: Visibility = {
    hidden: () => hidden,
    onChange: (cb) => {
      visibilityListeners.add(cb);
      return () => visibilityListeners.delete(cb);
    },
  };
  return {
    ticker: new Ticker(timers, visibility),
    pending,
    visibilityListeners,
    advance(ms: number) {
      const end = time + ms;
      for (;;) {
        const due = [...pending.entries()]
          .filter(([, t]) => t.at <= end)
          .sort((a, b) => a[1].at - b[1].at)[0];
        if (!due) break;
        pending.delete(due[0]);
        time = due[1].at;
        due[1].cb();
      }
      time = end;
    },
    setHidden(h: boolean) {
      hidden = h;
      for (const cb of visibilityListeners) cb();
    },
  };
}

describe("Ticker", () => {
  it("does not run without subscribers", () => {
    const h = harness();
    expect(h.ticker.running).toBe(false);
    expect(h.pending.size).toBe(0);
  });

  it("ticks on whole seconds with a single timer for all subscribers", () => {
    const h = harness();
    const a: number[] = [];
    const b: number[] = [];
    h.ticker.subscribe((t) => a.push(t));
    h.ticker.subscribe((t) => b.push(t));
    expect(h.pending.size).toBe(1);
    h.advance(2_000);
    expect(a).toEqual([11_000, 12_000]);
    expect(b).toEqual(a);
  });

  it("stops when the last subscriber leaves", () => {
    const h = harness();
    const off1 = h.ticker.subscribe(() => {});
    const off2 = h.ticker.subscribe(() => {});
    off1();
    expect(h.ticker.running).toBe(true);
    off2();
    expect(h.ticker.running).toBe(false);
    expect(h.pending.size).toBe(0);
    expect(h.visibilityListeners.size).toBe(0);
  });

  it("stops while the page is hidden and catches up when shown", () => {
    const h = harness();
    const seen: number[] = [];
    h.ticker.subscribe((t) => seen.push(t));
    h.setHidden(true);
    expect(h.ticker.running).toBe(false);
    h.advance(60_000);
    expect(seen).toEqual([]);
    h.setHidden(false);
    expect(seen).toEqual([70_250]);
    expect(h.ticker.running).toBe(true);
  });
});

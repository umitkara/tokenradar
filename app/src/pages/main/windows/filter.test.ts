import { describe, expect, it } from "vitest";
import {
  DEFAULT_PATTERNS_QUERY,
  DEFAULT_WINDOWS_QUERY,
  parsePatternsQuery,
  parseWindowsQuery,
  spanRange,
  toPatternsQuery,
  toWindowsQuery,
} from "./filter";

const DAY = 86_400_000;

describe("windows query", () => {
  it("writes nothing for the defaults", () => {
    expect(toWindowsQuery(DEFAULT_WINDOWS_QUERY)).toBe("");
    expect(parseWindowsQuery("")).toEqual(DEFAULT_WINDOWS_QUERY);
  });

  it("round-trips a tab and a custom range", () => {
    const q = {
      tab: "list" as const,
      range: "custom" as const,
      from: "2026-11-01",
      to: "2026-11-14",
    };
    const text = toWindowsQuery(q);
    expect(text).toBe("tab=list&range=custom&from=2026-11-01&to=2026-11-14");
    expect(parseWindowsQuery(text)).toEqual(q);
  });

  it("falls back on unreadable values", () => {
    expect(parseWindowsQuery("tab=chart&range=9w")).toEqual(DEFAULT_WINDOWS_QUERY);
    // A custom range without both days is the default range.
    expect(parseWindowsQuery("range=custom&from=2026-11-01").range).toBe("12w");
  });

  it("orders swapped custom days", () => {
    const q = parseWindowsQuery("range=custom&from=2026-11-14&to=2026-11-01");
    expect([q.from, q.to]).toEqual(["2026-11-01", "2026-11-14"]);
  });
});

describe("patterns query", () => {
  it("round-trips the metric", () => {
    expect(toPatternsQuery(DEFAULT_PATTERNS_QUERY)).toBe("");
    const q = { ...DEFAULT_PATTERNS_QUERY, range: "4w" as const, metric: "percent" as const };
    expect(toPatternsQuery(q)).toBe("range=4w&metric=percent");
    expect(parsePatternsQuery("range=4w&metric=percent")).toEqual(q);
    expect(parsePatternsQuery("metric=joules").metric).toBe("cost");
  });
});

describe("spanRange", () => {
  const now = new Date(2026, 10, 21, 12, 0);

  it("counts presets back from now", () => {
    expect(spanRange({ range: "4w", from: null, to: null }, now)).toEqual({
      from: now.getTime() - 28 * DAY,
      to: now.getTime(),
    });
    expect(spanRange({ range: "all", from: null, to: null }, now).from).toBe(0);
  });

  it("includes the whole last custom day", () => {
    const r = spanRange({ range: "custom", from: "2026-11-01", to: "2026-11-02" }, now);
    expect(r.from).toBe(new Date(2026, 10, 1).getTime());
    expect(r.to).toBe(new Date(2026, 10, 3).getTime());
  });
});

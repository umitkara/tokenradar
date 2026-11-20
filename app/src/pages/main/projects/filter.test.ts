import { describe, expect, it } from "vitest";
import {
  DEFAULT_PROJECT_QUERY,
  type ProjectQuery,
  parseProjectQuery,
  toProjectFilter,
  toProjectQuery,
} from "./filter";

const q = (over: Partial<ProjectQuery>): ProjectQuery => ({ ...DEFAULT_PROJECT_QUERY, ...over });

describe("toProjectQuery / parseProjectQuery", () => {
  it("writes nothing for the defaults", () => {
    expect(toProjectQuery(DEFAULT_PROJECT_QUERY)).toBe("");
    expect(parseProjectQuery("")).toEqual(DEFAULT_PROJECT_QUERY);
  });

  it("round-trips every field in a fixed order", () => {
    const f = q({
      range: "custom",
      from: "2026-11-01",
      to: "2026-11-10",
      q: "web/api",
      sort: "impact",
      view: "tree",
      showTemp: true,
    });
    const text = toProjectQuery(f);
    expect(text).toBe(
      "range=custom&from=2026-11-01&to=2026-11-10&q=web%2Fapi&sort=impact&view=tree&temp=1",
    );
    expect(parseProjectQuery(text)).toEqual(f);
  });

  it("falls back to defaults for unknown values", () => {
    expect(parseProjectQuery("sort=size&view=grid&temp=yes&range=2y")).toEqual(
      DEFAULT_PROJECT_QUERY,
    );
  });

  it("writes the 7-day range because the default is 30 days", () => {
    expect(toProjectQuery(q({ range: "7d" }))).toBe("range=7d");
    expect(parseProjectQuery("range=7d").range).toBe("7d");
  });
});

describe("toProjectFilter", () => {
  const now = new Date(2026, 10, 21, 12);

  it("hides temporary folders unless asked", () => {
    expect(toProjectFilter(DEFAULT_PROJECT_QUERY, now).hide_temp).toBe(true);
    expect(toProjectFilter(q({ showTemp: true }), now).hide_temp).toBe(false);
  });

  it("sends a trimmed search or none", () => {
    expect(toProjectFilter(q({ q: "  orbit " }), now).search).toBe("orbit");
    expect(toProjectFilter(q({ q: "   " }), now).search).toBeNull();
  });

  it("has no time range for all time", () => {
    expect(toProjectFilter(q({ range: "all" }), now).range).toBeNull();
    expect(toProjectFilter(DEFAULT_PROJECT_QUERY, now).range).not.toBeNull();
  });
});

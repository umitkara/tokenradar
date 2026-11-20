import { describe, expect, it } from "vitest";
import { DEFAULT_QUERY, parseQuery, type SessionQuery, toFilter, toQuery } from "./filter";

const q = (over: Partial<SessionQuery>): SessionQuery => ({ ...DEFAULT_QUERY, ...over });

describe("toQuery / parseQuery", () => {
  it("writes nothing for the defaults", () => {
    expect(toQuery(DEFAULT_QUERY)).toBe("");
    expect(parseQuery("")).toEqual(DEFAULT_QUERY);
  });

  it("round-trips every field in a fixed order", () => {
    const f = q({
      range: "custom",
      from: "2026-11-01",
      to: "2026-11-10",
      project: 12,
      family: "opus",
      live: true,
      q: "a b&c",
      sort: "cost",
    });
    const text = toQuery(f);
    expect(text).toBe(
      "range=custom&from=2026-11-01&to=2026-11-10&project=12&model=opus&live=1&q=a%20b%26c&sort=cost",
    );
    expect(parseQuery(text)).toEqual(f);
  });

  it("falls back to defaults for unknown or malformed values", () => {
    expect(parseQuery("range=year&model=gpt&sort=x&project=-3&live=yes")).toEqual(DEFAULT_QUERY);
    expect(parseQuery("project=1.5").project).toBeNull();
    // A custom range needs two real days.
    expect(parseQuery("range=custom&from=2026-02-30&to=2026-03-01")).toEqual(DEFAULT_QUERY);
    expect(parseQuery("range=custom&from=2026-11-01").range).toBe("30d");
  });

  it("orders a reversed custom range", () => {
    const f = parseQuery("range=custom&from=2026-11-10&to=2026-11-01");
    expect([f.from, f.to]).toEqual(["2026-11-01", "2026-11-10"]);
  });

  it("drops a custom range without days and blank search text", () => {
    expect(toQuery(q({ range: "custom" }))).toBe("");
    expect(toQuery(q({ q: "   " }))).toBe("");
  });
});

describe("toFilter", () => {
  const now = new Date(2026, 10, 20, 15, 30);

  it("counts presets back from now and today from local midnight", () => {
    expect(toFilter(q({ range: "7d" }), now).range).toEqual({
      from: now.getTime() - 7 * 86_400_000,
      to: now.getTime(),
    });
    expect(toFilter(q({ range: "today" }), now).range).toEqual({
      from: new Date(2026, 10, 20).getTime(),
      to: now.getTime(),
    });
    expect(toFilter(q({ range: "all" }), now).range).toBeNull();
  });

  it("covers whole local days of a custom range", () => {
    const f = toFilter(q({ range: "custom", from: "2026-11-01", to: "2026-11-10" }), now);
    expect(f.range).toEqual({
      from: new Date(2026, 10, 1).getTime(),
      to: new Date(2026, 10, 11).getTime(),
    });
  });

  it("trims the search, keeps the rest and lets a locked project win", () => {
    const f = toFilter(q({ q: "  api ", project: 3, family: "sonnet", live: true }), now, 9);
    expect(f).toMatchObject({ search: "api", project: 9, family: "sonnet", live_only: true });
    expect(toFilter(q({ q: " " }), now).search).toBeNull();
  });
});

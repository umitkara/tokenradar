import { describe, expect, it } from "vitest";
import { DEFAULT_ROUTE, hrefFor, parseHash, ROUTES, shortcutFor } from "./router";

const key = (
  k: string,
  mods: Partial<Record<"ctrlKey" | "altKey" | "shiftKey" | "metaKey", boolean>> = {},
) => ({
  key: k,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...mods,
});

describe("parseHash", () => {
  it("reads a route with or without the hash sign", () => {
    expect(parseHash("#sessions")).toEqual({ route: "sessions", param: null, query: "" });
    expect(parseHash("windows")).toEqual({ route: "windows", param: null, query: "" });
    expect(parseHash("#Projects")).toEqual({ route: "projects", param: null, query: "" });
  });

  it("falls back to the overview", () => {
    for (const hash of ["", "#", "#settings", "#overview-x", "#/sessions"]) {
      expect(parseHash(hash)).toEqual({ route: DEFAULT_ROUTE, param: null, query: "" });
    }
  });

  it("keeps a parameter after the route", () => {
    expect(parseHash("#sessions/42")).toEqual({ route: "sessions", param: "42", query: "" });
    expect(parseHash("#sessions/")).toEqual({ route: "sessions", param: null, query: "" });
    expect(parseHash("#projects/a%2Fb")).toEqual({ route: "projects", param: "a/b", query: "" });
  });

  it("splits off a query, which a detail page and an unknown route drop or keep", () => {
    expect(parseHash("#sessions?range=7d&q=a%20b")).toEqual({
      route: "sessions",
      param: null,
      query: "range=7d&q=a%20b",
    });
    expect(parseHash("#sessions/42?x=1")).toEqual({ route: "sessions", param: "42", query: "x=1" });
    expect(parseHash("#nope?x=1")).toEqual({ route: DEFAULT_ROUTE, param: null, query: "" });
    expect(parseHash("#sessions/%E0")).toEqual({ route: "sessions", param: "%E0", query: "" });
    expect(hrefFor("sessions", null, "live=1")).toBe("#sessions?live=1");
  });

  it("round-trips hrefFor", () => {
    for (const route of ROUTES) {
      expect(parseHash(hrefFor(route))).toEqual({ route, param: null, query: "" });
      expect(parseHash(hrefFor(route, "x/y z"))).toEqual({ route, param: "x/y z", query: "" });
    }
  });
});

describe("shortcutFor", () => {
  it("maps Ctrl+1..5 to the pages in order", () => {
    ROUTES.forEach((route, i) => {
      expect(shortcutFor(key(String(i + 1), { ctrlKey: true }))).toEqual({ kind: "route", route });
    });
  });

  it("maps Ctrl+, to the settings", () => {
    expect(shortcutFor(key(",", { ctrlKey: true }))).toEqual({ kind: "settings" });
  });

  it("ignores other keys and modifiers", () => {
    expect(shortcutFor(key("1"))).toBeNull();
    expect(shortcutFor(key("6", { ctrlKey: true }))).toBeNull();
    expect(shortcutFor(key("0", { ctrlKey: true }))).toBeNull();
    expect(shortcutFor(key("1", { ctrlKey: true, shiftKey: true }))).toBeNull();
    expect(shortcutFor(key("1", { ctrlKey: true, altKey: true }))).toBeNull();
    expect(shortcutFor(key(",", { metaKey: true }))).toBeNull();
  });
});

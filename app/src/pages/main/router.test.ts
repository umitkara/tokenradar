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
    expect(parseHash("#sessions")).toEqual({ route: "sessions", param: null });
    expect(parseHash("windows")).toEqual({ route: "windows", param: null });
    expect(parseHash("#Projects")).toEqual({ route: "projects", param: null });
  });

  it("falls back to the overview", () => {
    for (const hash of ["", "#", "#settings", "#overview-x", "#/sessions"]) {
      expect(parseHash(hash)).toEqual({ route: DEFAULT_ROUTE, param: null });
    }
  });

  it("keeps a parameter after the route", () => {
    expect(parseHash("#sessions/42")).toEqual({ route: "sessions", param: "42" });
    expect(parseHash("#sessions/")).toEqual({ route: "sessions", param: null });
    expect(parseHash("#projects/a%2Fb")).toEqual({ route: "projects", param: "a/b" });
  });

  it("round-trips hrefFor", () => {
    for (const route of ROUTES) {
      expect(parseHash(hrefFor(route))).toEqual({ route, param: null });
      expect(parseHash(hrefFor(route, "x/y z"))).toEqual({ route, param: "x/y z" });
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

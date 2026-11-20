// The main window's hash router, without a library: `#sessions`, `#sessions/<param>`, and either
// with a `?query` that a page keeps its filters in (`#sessions?range=7d`). The
// route names are the core's `MainRoute` (crates/tokenradar-core/src/deeplink.rs), so a link
// from the CLI or the panel lands on the same page. Pure functions; the reactive half is
// `state.svelte.ts`.

import type { MainRoute } from "../../lib/ipc/bindings";

/** In navigation order: `Ctrl+1` is the first. */
export const ROUTES: readonly MainRoute[] = [
  "overview",
  "windows",
  "sessions",
  "projects",
  "patterns",
];

export const DEFAULT_ROUTE: MainRoute = "overview";

export interface Location {
  route: MainRoute;
  /** What follows the route (`#sessions/42` → `"42"`); a detail page reads it. */
  param: string | null;
  /** The text after `?`, without it (`""` when there is none); a list page's filters. */
  query: string;
}

export function isRoute(name: string): name is MainRoute {
  return (ROUTES as readonly string[]).includes(name);
}

/** `location.hash` (with or without `#`) → a page; anything unknown is the default page. */
export function parseHash(hash: string): Location {
  const full = hash.startsWith("#") ? hash.slice(1) : hash;
  const mark = full.indexOf("?");
  const text = mark < 0 ? full : full.slice(0, mark);
  const query = mark < 0 ? "" : full.slice(mark + 1);
  const slash = text.indexOf("/");
  const name = (slash < 0 ? text : text.slice(0, slash)).toLowerCase();
  if (!isRoute(name)) return { route: DEFAULT_ROUTE, param: null, query: "" };
  const rest = slash < 0 ? "" : safeDecode(text.slice(slash + 1));
  return { route: name, param: rest === "" ? null : rest, query };
}

/** A malformed escape (`%E0`) is kept as written instead of throwing. */
function safeDecode(s: string): string {
  try {
    return decodeURIComponent(s);
  } catch {
    return s;
  }
}

export function hrefFor(route: MainRoute, param: string | null = null, query = ""): string {
  const base = param === null ? `#${route}` : `#${route}/${encodeURIComponent(param)}`;
  return query === "" ? base : `${base}?${query}`;
}

export type Shortcut = { kind: "route"; route: MainRoute } | { kind: "settings" };

/** `Ctrl+1..5` switch pages, `Ctrl+,` opens the settings. Other modifiers do not count. */
export function shortcutFor(e: {
  key: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}): Shortcut | null {
  if (!e.ctrlKey || e.altKey || e.shiftKey || e.metaKey) return null;
  if (e.key === ",") return { kind: "settings" };
  const n = Number(e.key);
  if (Number.isInteger(n) && n >= 1 && n <= ROUTES.length) {
    const route = ROUTES[n - 1];
    if (route) return { kind: "route", route };
  }
  return null;
}

/** The message key of a page's name. */
export function navKey(route: MainRoute): string {
  return `main-nav-${route}`;
}

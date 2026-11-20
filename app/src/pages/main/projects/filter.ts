// The projects list's filters in the URL fragment (Step 16b): `#projects?range=all&q=text
// &sort=cost&view=tree&temp=1`. Same rules as the sessions list (`sessions/filter.ts`): only
// non-default values are written, in a fixed order, and anything unreadable falls back to its
// default. Temporary folders are hidden by default, so `temp=1` means "show them".

import type { ProjectFilter, ProjectSort } from "../../../lib/ipc/bindings";
import {
  oneOf,
  parseRange,
  queryText,
  type RangeQuery,
  rangeFilter,
  writeRange,
} from "../sessions/filter";

export const PROJECT_SORTS: readonly ProjectSort[] = ["last_active", "cost", "impact", "sessions"];
export type ProjectView = "list" | "tree";
export const PROJECT_VIEWS: readonly ProjectView[] = ["list", "tree"];

export interface ProjectQuery extends RangeQuery {
  q: string;
  sort: ProjectSort;
  view: ProjectView;
  /** Show projects in temporary folders too. */
  showTemp: boolean;
}

export const DEFAULT_PROJECT_QUERY: ProjectQuery = {
  range: "30d",
  from: null,
  to: null,
  q: "",
  sort: "last_active",
  view: "list",
  showTemp: false,
};

export function parseProjectQuery(text: string): ProjectQuery {
  const p = new URLSearchParams(text);
  return {
    ...parseRange(p, DEFAULT_PROJECT_QUERY.range),
    q: p.get("q") ?? "",
    sort: oneOf(p.get("sort"), PROJECT_SORTS) ?? DEFAULT_PROJECT_QUERY.sort,
    view: oneOf(p.get("view"), PROJECT_VIEWS) ?? DEFAULT_PROJECT_QUERY.view,
    showTemp: p.get("temp") === "1",
  };
}

export function toProjectQuery(f: ProjectQuery): string {
  const p = new URLSearchParams();
  writeRange(p, f, DEFAULT_PROJECT_QUERY.range);
  if (f.q.trim() !== "") p.set("q", f.q);
  if (f.sort !== DEFAULT_PROJECT_QUERY.sort) p.set("sort", f.sort);
  if (f.view !== DEFAULT_PROJECT_QUERY.view) p.set("view", f.view);
  if (f.showTemp) p.set("temp", "1");
  return queryText(p);
}

export function toProjectFilter(f: ProjectQuery, now: Date): ProjectFilter {
  const search = f.q.trim();
  return {
    range: rangeFilter(f, now),
    search: search === "" ? null : search,
    hide_temp: !f.showTemp,
  };
}

/** The list's last query text, for "Back to projects" from a detail. */
export const projectListMemory = { query: "" };

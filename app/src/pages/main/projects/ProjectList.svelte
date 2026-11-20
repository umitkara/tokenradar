<!--
  The projects list (Step 16b): every project with usage in the range, as a flat list (keyset
  pages) or grouped by folder (the backend's tree with totals per folder). Filters live in the
  URL fragment (`filter.ts`), as on the sessions list.

  - Temporary folders are hidden by default; the backend applies the patterns from the settings
    and says how many projects it hid, and a link shows them.
  - Tree: a folder row opens and closes with a click, Enter, → and ←; a project row opens its
    detail. ← on a project moves to its folder. The open state lives here only.
  - Refreshes while visible on new usage, at most every 10 s.
-->
<script lang="ts">
import { onMount, untrack } from "svelte";
import EmptyState from "../../../lib/components/EmptyState.svelte";
import Segmented from "../../../lib/components/Segmented.svelte";
import Select from "../../../lib/components/Select.svelte";
import Skeleton from "../../../lib/components/Skeleton.svelte";
import Sparkline from "../../../lib/components/Sparkline.svelte";
import Toggle from "../../../lib/components/Toggle.svelte";
import VirtualList from "../../../lib/components/VirtualList.svelte";
import { locale, t } from "../../../lib/i18n/i18n.svelte";
import {
  type CoreEvent,
  commands,
  type ProjectNode,
  type ProjectRow,
  type ProjectSort,
} from "../../../lib/ipc/bindings";
import { subscribeCoreEvents } from "../../../lib/ipc/core-events";
import { inTauri } from "../../../lib/ipc/runtime";
import { clock } from "../../../lib/time/clock.svelte";
import { usd } from "../../panel/detail";
import { hrefFor } from "../router";
import { formatDay, RANGE_PRESETS, type RangePreset } from "../sessions/filter";
import { middleEllipsis } from "../sessions/view";
import {
  DEFAULT_PROJECT_QUERY,
  PROJECT_SORTS,
  type ProjectQuery,
  parseProjectQuery,
  projectListMemory,
  toProjectFilter,
  toProjectQuery,
} from "./filter";
import { parentIndex, visibleNodes } from "./view";

interface Props {
  query: string;
  /** Synthetic data for the gallery; skips the backend. */
  preset?: { rows: ProjectRow[]; nodes: ProjectNode[]; hidden: number | null } | null;
}

let { query, preset = null }: Props = $props();

const PAGE = 100;
const MIN_GAP_MS = 10_000;
const RELEVANT = new Set<CoreEvent["type"]>(["usage_appended", "live_sessions_changed"]);
const COLUMNS = "minmax(0, 2fr) minmax(0, 2.2fr) 4.5em 7em 6em 5.5em 64px";

let current = $state<ProjectQuery>(untrack(() => parseProjectQuery(query)));
let searchText = $state(untrack(() => current.q));
let rows = $state<ProjectRow[]>(untrack(() => preset?.rows) ?? []);
let nodes = $state<ProjectNode[]>(untrack(() => preset?.nodes) ?? []);
let hidden = $state<number | null>(untrack(() => preset?.hidden) ?? null);
let cursor = $state<string | null>(null);
let collapsed = $state(new Set<string>());
let loaded = $state(untrack(() => preset) != null || !inTauri());
let loading = $state(false);
let failed = $state(false);
let search: HTMLInputElement;
let list: ReturnType<typeof VirtualList<ProjectRow>> | undefined = $state();
let treeList: ReturnType<typeof VirtualList<ProjectNode>> | undefined = $state();
let generation = 0;
let lastLoad = 0;
let pending: ReturnType<typeof setTimeout> | null = null;
let debounce: ReturnType<typeof setTimeout> | null = null;

const visible = $derived(visibleNodes(nodes, collapsed));

$effect(() => {
  const next = parseProjectQuery(query);
  projectListMemory.query = query;
  current = next;
  untrack(() => {
    if (next.q !== searchText.trim()) searchText = next.q;
  });
});

$effect(() => {
  const q = current;
  if (preset) return;
  list?.reset();
  treeList?.reset();
  void loadFirst(q);
});

async function loadFirst(q: ProjectQuery, keep = 0) {
  const mine = ++generation;
  loading = true;
  lastLoad = Date.now();
  const filter = toProjectFilter(q, new Date());
  if (q.view === "tree") {
    const result = await commands.projectTree(filter, q.sort);
    if (mine !== generation) return;
    finish(result.status === "ok");
    if (result.status === "ok") {
      nodes = result.data.nodes;
      hidden = result.data.hidden;
    }
    return;
  }
  const limit = Math.min(200, Math.max(PAGE, keep));
  const result = await commands.listProjects(filter, q.sort, null, limit);
  if (mine !== generation) return;
  finish(result.status === "ok");
  if (result.status === "ok") {
    rows = result.data.items;
    cursor = result.data.next;
    hidden = result.data.hidden;
  }
}

function finish(ok: boolean) {
  loading = false;
  loaded = true;
  if (ok) failed = false;
  else if (rows.length === 0 && nodes.length === 0) failed = true;
}

async function loadMore() {
  if (!cursor || loading || current.view !== "list") return;
  const mine = generation;
  loading = true;
  const result = await commands.listProjects(
    toProjectFilter(current, new Date()),
    current.sort,
    cursor,
    PAGE,
  );
  if (mine !== generation) return;
  loading = false;
  if (result.status === "ok") {
    const ids = new Set(rows.map((r) => r.id));
    rows = [...rows, ...result.data.items.filter((r) => !ids.has(r.id))];
    cursor = result.data.next;
  }
}

function schedule() {
  if (pending || preset) return;
  const wait = Math.max(0, lastLoad + MIN_GAP_MS - Date.now());
  pending = setTimeout(() => {
    pending = null;
    void loadFirst(current, rows.length);
  }, wait);
}

function apply(change: Partial<ProjectQuery>) {
  location.hash = hrefFor("projects", null, toProjectQuery({ ...current, ...change }));
}

function onsearch() {
  if (debounce) clearTimeout(debounce);
  debounce = setTimeout(() => {
    debounce = null;
    if (searchText.trim() === current.q.trim()) return;
    const next = { ...current, q: searchText };
    const text = toProjectQuery(next);
    history.replaceState(history.state, "", hrefFor("projects", null, text));
    projectListMemory.query = text;
    current = next;
  }, 200);
}

function setRange(range: RangePreset) {
  if (range !== "custom") {
    apply({ range, from: null, to: null });
    return;
  }
  const today = new Date();
  const weekAgo = new Date(today);
  weekAgo.setDate(today.getDate() - 6);
  apply({ range, from: current.from ?? formatDay(weekAgo), to: current.to ?? formatDay(today) });
}

function setDay(which: "from" | "to", value: string) {
  if (value) apply({ [which]: value });
}

function onkeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && !e.altKey && e.key.toLowerCase() === "f") {
    e.preventDefault();
    search.focus();
    search.select();
  }
}

function open(id: number) {
  location.hash = hrefFor("projects", String(id));
}

function toggle(node: ProjectNode, open?: boolean) {
  const next = new Set(collapsed);
  const close = open === undefined ? !next.has(node.path) : !open;
  if (close) next.add(node.path);
  else next.delete(node.path);
  collapsed = next;
}

function activateNode(node: ProjectNode) {
  if (node.children > 0 && !node.project) toggle(node);
  else if (node.project) open(node.project.id);
}

function treeKey(key: string, node: ProjectNode, index: number): boolean {
  if (key === "ArrowRight" && node.children > 0 && collapsed.has(node.path)) {
    toggle(node, true);
    return true;
  }
  if (key === "ArrowLeft") {
    if (node.children > 0 && !collapsed.has(node.path)) {
      toggle(node, false);
      return true;
    }
    const parent = parentIndex(visible, index);
    if (parent != null) {
      treeList?.select(parent);
      return true;
    }
  }
  return false;
}

function money(micro: number): string {
  return t("sessions-usd", { usd: usd(micro) });
}

function ago(ms: number | null): string {
  if (ms == null) return t("value-none");
  const d = clock.now - ms;
  return d < 60_000 ? t("sessions-just-now") : t("sessions-ago", { ms: d });
}

function weekly(pct: number | null): string {
  return pct == null ? t("value-none") : t("value-pct", { pct, estimate: "yes" });
}

function trendLabel(values: readonly number[]): string {
  return t("projects-trend-tip", { usd: usd(values.reduce((a, b) => a + b, 0)) });
}

const filtered = $derived(
  toProjectQuery({ ...current, sort: DEFAULT_PROJECT_QUERY.sort, view: "list" }) !== "",
);
const empty = $derived(current.view === "tree" ? nodes.length === 0 : rows.length === 0);
const rangeOptions = $derived(
  RANGE_PRESETS.map((value) => ({ value, label: t(`sessions-range-${value}`) })),
);
const sortOptions = $derived(
  PROJECT_SORTS.map((value) => ({ value, label: t(`projects-sort-${value.replace("_", "-")}`) })),
);

onMount(() => {
  if (preset) return;
  const stop = subscribeCoreEvents({
    onEvent: (e) => {
      if (RELEVANT.has(e.type)) schedule();
    },
    onResume: () => {
      if (lastLoad > 0) void loadFirst(current, rows.length);
    },
  });
  return () => {
    stop();
    if (pending) clearTimeout(pending);
    if (debounce) clearTimeout(debounce);
  };
});
</script>

<svelte:window {onkeydown} />

{#snippet header()}
  <span role="columnheader">{t("projects-col-name")}</span>
  <span role="columnheader">{t("projects-col-path")}</span>
  <span role="columnheader" class="num">{t("projects-col-sessions")}</span>
  <span role="columnheader">{t("projects-col-last")}</span>
  <span role="columnheader" class="num">{t("projects-col-cost")}</span>
  <span role="columnheader" class="num" title={t("projects-weekly-tip")}>{t("projects-col-weekly")}</span>
  <span role="columnheader">{t("projects-col-trend")}</span>
{/snippet}

{#snippet numbers(sessions: number, last: number | null, cost: number, pct: number | null, trend: readonly number[])}
  <span role="gridcell" class="num readout">{sessions}</span>
  <span
    role="gridcell"
    class="muted"
    title={last != null ? new Date(last).toLocaleString(locale()) : undefined}
  >
    {ago(last)}
  </span>
  <span role="gridcell" class="num readout">{money(cost)}</span>
  <span role="gridcell" class="num readout dim" title={t("projects-weekly-tip")}>{weekly(pct)}</span>
  <span role="gridcell" title={trendLabel(trend)}><Sparkline values={trend} /></span>
{/snippet}

<div class="page">
  <search class="filters" aria-label={t("sessions-filters")}>
    <input
      bind:this={search}
      bind:value={searchText}
      oninput={onsearch}
      type="search"
      class="search"
      aria-label={t("projects-search")}
      aria-keyshortcuts="Control+F"
      placeholder={t("projects-search-placeholder")}
    />
    <Segmented value={current.range} options={rangeOptions} label={t("sessions-range")} onchange={setRange} />
    {#if current.range === "custom"}
      <label class="day">
        <span>{t("sessions-from")}</span>
        <input type="date" value={current.from ?? ""} onchange={(e) => setDay("from", e.currentTarget.value)} />
      </label>
      <label class="day">
        <span>{t("sessions-to")}</span>
        <input type="date" value={current.to ?? ""} onchange={(e) => setDay("to", e.currentTarget.value)} />
      </label>
    {/if}
    <Toggle
      checked={current.view === "tree"}
      label={t("projects-group")}
      onchange={(on) => apply({ view: on ? "tree" : "list" })}
    />
    <Toggle
      checked={!current.showTemp}
      label={t("projects-hide-temp")}
      onchange={(hide) => apply({ showTemp: !hide })}
    />
    <Select
      value={current.sort}
      options={sortOptions}
      label={t("sessions-sort")}
      onchange={(sort: ProjectSort) => apply({ sort })}
    />
  </search>

  {#if hidden != null && hidden > 0}
    <p class="note">
      {t("projects-hidden", { n: hidden })}
      <a href={hrefFor("projects", null, toProjectQuery({ ...current, showTemp: true }))}>{t("projects-show-temp")}</a>
    </p>
  {/if}

  {#if !loaded}
    <Skeleton width="100%" height="320px" />
  {:else if failed}
    <p class="error" role="alert">{t("main-load-failed")}</p>
  {:else if empty && !loading}
    {#if filtered}
      <EmptyState title={t("projects-empty-filtered")} body={t("projects-empty-filtered-body")}>
        {#snippet action()}
          <a class="clear" href={hrefFor("projects")}>{t("sessions-clear")}</a>
        {/snippet}
      </EmptyState>
    {:else}
      <EmptyState title={t("projects-empty")} body={t("projects-empty-body")} />
    {/if}
  {:else if current.view === "tree"}
    <VirtualList
      bind:this={treeList}
      items={visible}
      key={(n) => n.path}
      label={t("projects-tree")}
      columns={COLUMNS}
      {header}
      onactivate={activateNode}
      onkey={treeKey}
    >
      {#snippet row(n: ProjectNode)}
        {@const folder = n.children > 0}
        {@const open = folder && !collapsed.has(n.path)}
        <span role="gridcell" class="name" style:padding-inline-start="{n.depth * 16}px">
          <!-- The keyboard opens and closes folders with → and ← on the grid. -->
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <span
            class="twisty"
            aria-hidden="true"
            onclick={(e) => {
              if (!folder) return;
              e.stopPropagation();
              toggle(n);
            }}
          >{folder ? (open ? "▾" : "▸") : ""}</span>
          {#if folder}
            <span class="visually-hidden">{t(open ? "projects-expanded" : "projects-collapsed")}</span>
          {/if}
          {#if n.project}
            <a class="title" href={hrefFor("projects", String(n.project.id))} title={n.path} tabindex="-1">
              {n.label}
            </a>
          {:else}
            <span class="title folder" title={n.path}>{n.label}</span>
          {/if}
        </span>
        <span role="gridcell" class="muted path" class:count={folder}>
          {folder ? t("projects-count", { n: n.projects }) : ""}
        </span>
        {@render numbers(n.sessions, n.last_active, n.cost, n.weekly_percent, n.trend)}
      {/snippet}
    </VirtualList>
  {:else}
    <VirtualList
      bind:this={list}
      items={rows}
      key={(r) => r.id}
      label={t("projects-list")}
      columns={COLUMNS}
      {header}
      hasMore={cursor != null}
      {loading}
      onmore={loadMore}
      onactivate={(r) => open(r.id)}
    >
      {#snippet row(r: ProjectRow)}
        <span role="gridcell" class="name">
          <span class="title" title={r.name}>{middleEllipsis(r.name, 48)}</span>
        </span>
        <span role="gridcell" class="muted path" title={r.path}>{middleEllipsis(r.path, 60)}</span>
        {@render numbers(r.sessions, r.last_active, r.cost, r.weekly_percent, r.trend)}
      {/snippet}
    </VirtualList>
  {/if}
  {#if loading && loaded}
    <p class="visually-hidden" role="status">{t("projects-loading")}</p>
  {/if}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    flex: 1 1 0;
    min-height: 0;
  }
  .filters {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-3);
  }
  .search {
    flex: 1 1 220px;
    max-width: 320px;
    min-height: 32px;
    padding: 0 var(--space-3);
    border-radius: var(--radius-control);
    border: 1px solid var(--stroke);
    background: var(--surface-1);
    color: var(--text-1);
    font: var(--type-body);
  }
  .search::placeholder {
    color: var(--text-3);
  }
  .day {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font: var(--type-caption);
    color: var(--text-2);
  }
  .day input {
    min-height: 32px;
    padding: 0 var(--space-2);
    border-radius: var(--radius-control);
    border: 1px solid var(--stroke);
    background: var(--surface-1);
    color: var(--text-1);
    font: var(--type-body);
    color-scheme: inherit;
  }
  .note {
    margin: 0;
    font: var(--type-caption);
    color: var(--text-2);
  }
  .note a,
  .clear {
    color: var(--text-1);
  }
  .num {
    text-align: end;
    white-space: nowrap;
  }
  .muted {
    color: var(--text-2);
    font: var(--type-caption);
  }
  .dim {
    color: var(--text-2);
  }
  .name {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    min-width: 0;
    font: var(--type-body);
    color: var(--text-1);
  }
  .twisty {
    flex: none;
    width: 1em;
    color: var(--text-2);
  }
  .title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: inherit;
    text-decoration: none;
  }
  a.title:hover {
    text-decoration: underline;
  }
  .folder {
    color: var(--text-2);
  }
  .path {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
  }
  /* A folder's project count sits in the path column but is not a path. */
  .path.count {
    font-family: inherit;
  }
  .readout {
    font-size: 12px;
  }
  .error {
    padding: var(--space-2) var(--space-3);
    border-left: 2px solid var(--signal-crit);
    background: var(--surface-1);
    border-radius: var(--radius-control);
  }
</style>

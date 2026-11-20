<!--
  The sessions list (Step 16a): every session tree with usage in the range, newest activity
  first, 10,000 rows as smooth as 30 (`VirtualList`). Filters live in the URL fragment
  (`filter.ts`), so back and forward step through them and a link reopens the same list.

  - Search is debounced (200 ms) and replaces the fragment instead of pushing one entry per key.
    `Ctrl+F` focuses it.
  - Refreshes while visible on running-session changes and new usage (at most every 10 s),
    keeping the rows already loaded.
  - The project picker is a native select: it finds a project by typing its first letters.
-->
<script lang="ts">
import { onMount, untrack } from "svelte";
import EmptyState from "../../../lib/components/EmptyState.svelte";
import Segmented from "../../../lib/components/Segmented.svelte";
import Select from "../../../lib/components/Select.svelte";
import Skeleton from "../../../lib/components/Skeleton.svelte";
import Toggle from "../../../lib/components/Toggle.svelte";
import VirtualList from "../../../lib/components/VirtualList.svelte";
import { dur } from "../../../lib/i18n/format";
import { locale, t } from "../../../lib/i18n/i18n.svelte";
import {
  type CoreEvent,
  commands,
  type ModelFamily,
  type ProjectRef,
  type SessionRow,
  type SessionSort,
} from "../../../lib/ipc/bindings";
import { subscribeCoreEvents } from "../../../lib/ipc/core-events";
import { inTauri } from "../../../lib/ipc/runtime";
import { clock } from "../../../lib/time/clock.svelte";
import { usd } from "../../panel/detail";
import { countOf } from "../overview/view";
import { hrefFor } from "../router";
import Badge from "./Badge.svelte";
import {
  DEFAULT_QUERY,
  FAMILIES,
  formatDay,
  listMemory,
  parseQuery,
  RANGE_PRESETS,
  type RangePreset,
  type SessionQuery,
  SORTS,
  toFilter,
  toQuery,
} from "./filter";
import { durationOf, impactOf, middleEllipsis, modelLabel } from "./view";

interface Props {
  query: string;
  /** Synthetic rows for the gallery; skips the backend. */
  preset?: SessionRow[] | null;
}

let { query, preset = null }: Props = $props();

const PAGE = 100;
const MIN_GAP_MS = 10_000;
const RELEVANT = new Set<CoreEvent["type"]>(["live_sessions_changed", "usage_appended"]);
const COLUMNS = "10px minmax(0, 2.6fr) 8em 6.5em minmax(0, 1.1fr) 5.5em 5em 5.5em 3em";

// Seeded once; the effect below follows later fragment changes.
let current = $state<SessionQuery>(untrack(() => parseQuery(query)));
let searchText = $state(untrack(() => current.q));
let items = $state<SessionRow[]>(untrack(() => preset) ?? []);
let cursor = $state<string | null>(null);
let loaded = $state(untrack(() => preset) != null || !inTauri());
let loading = $state(false);
let failed = $state(false);
let projects = $state<ProjectRef[]>([]);
let search: HTMLInputElement;
let list: ReturnType<typeof VirtualList<SessionRow>> | undefined = $state();
let generation = 0;
let lastLoad = 0;
let pending: ReturnType<typeof setTimeout> | null = null;
let debounce: ReturnType<typeof setTimeout> | null = null;

// The fragment changed (a filter, back or forward): read it again.
$effect(() => {
  const next = parseQuery(query);
  listMemory.query = query;
  current = next;
  // Only the fragment drives this effect; typing must not re-run it.
  untrack(() => {
    if (next.q !== searchText.trim()) searchText = next.q;
  });
});

// New filters: start over from the first page.
$effect(() => {
  const q = current;
  if (preset) return;
  list?.reset();
  void loadFirst(q);
});

async function loadFirst(q: SessionQuery, keep = 0) {
  const mine = ++generation;
  loading = true;
  lastLoad = Date.now();
  const limit = Math.min(200, Math.max(PAGE, keep));
  const result = await commands.listSessions(toFilter(q, new Date()), q.sort, null, limit);
  if (mine !== generation) return;
  loading = false;
  loaded = true;
  if (result.status === "error") {
    if (items.length === 0) failed = true;
    return;
  }
  failed = false;
  const head = result.data.items;
  if (keep > head.length && result.data.next) {
    // A refresh beyond one page: keep the older rows that the new head does not repeat.
    const ids = new Set(head.map((r) => r.id));
    items = [...head, ...items.slice(head.length).filter((r) => !ids.has(r.id))];
    return;
  }
  items = head;
  cursor = result.data.next;
}

async function loadMore() {
  if (!cursor || loading) return;
  const mine = generation;
  loading = true;
  const result = await commands.listSessions(
    toFilter(current, new Date()),
    current.sort,
    cursor,
    PAGE,
  );
  if (mine !== generation) return;
  loading = false;
  if (result.status === "ok") {
    const ids = new Set(items.map((r) => r.id));
    items = [...items, ...result.data.items.filter((r) => !ids.has(r.id))];
    cursor = result.data.next;
  }
}

function schedule() {
  if (pending || preset) return;
  const wait = Math.max(0, lastLoad + MIN_GAP_MS - Date.now());
  pending = setTimeout(() => {
    pending = null;
    void loadFirst(current, items.length);
  }, wait);
}

async function loadProjects() {
  const result = await commands.listProjects(
    { range: null, search: null },
    "last_active",
    null,
    200,
  );
  if (result.status === "ok") {
    projects = result.data.items
      .map((p) => ({ id: p.id, name: p.name }))
      .sort((a, b) => a.name.localeCompare(b.name, locale()));
  }
}

/** A filter change: a new history entry (back returns to the previous filters). */
function apply(change: Partial<SessionQuery>) {
  const next = { ...current, ...change };
  location.hash = hrefFor("sessions", null, toQuery(next));
}

function onsearch() {
  if (debounce) clearTimeout(debounce);
  debounce = setTimeout(() => {
    debounce = null;
    if (searchText.trim() === current.q.trim()) return;
    const next = { ...current, q: searchText };
    const text = toQuery(next);
    // One entry per search, not per key: replace the fragment and read it ourselves.
    history.replaceState(history.state, "", hrefFor("sessions", null, text));
    listMemory.query = text;
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
  if (!value) return;
  apply({ [which]: value });
}

function onkeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && !e.altKey && e.key.toLowerCase() === "f") {
    e.preventDefault();
    search.focus();
    search.select();
  }
}

function open(row: SessionRow) {
  location.hash = hrefFor("sessions", String(row.id));
}

function money(micro: number): string {
  return t("sessions-usd", { usd: usd(micro) });
}

function tokensOf(row: SessionRow): string {
  const n = row.tokens.input + row.tokens.output + row.tokens.cache_read + row.tokens.cache_write;
  return t("overview-count", countOf(n));
}

function ago(ms: number | null): string {
  if (ms == null) return t("value-none");
  const d = clock.now - ms;
  return d < 60_000 ? t("sessions-just-now") : t("sessions-ago", { ms: d });
}

const filtered = $derived(toQuery({ ...current, sort: DEFAULT_QUERY.sort }) !== "");

const rangeOptions = $derived(
  RANGE_PRESETS.map((value) => ({ value, label: t(`sessions-range-${value}`) })),
);
const projectOptions = $derived([
  { value: "", label: t("sessions-project-all") },
  ...projects.map((p) => ({ value: String(p.id), label: p.name })),
]);
const familyOptions = $derived([
  { value: "", label: t("sessions-model-all") },
  ...FAMILIES.map((f) => ({ value: f, label: f.charAt(0).toUpperCase() + f.slice(1) })),
]);
const sortOptions = $derived(
  SORTS.map((value) => ({ value, label: t(`sessions-sort-${value.replace("_", "-")}`) })),
);

onMount(() => {
  if (preset) return;
  void loadProjects();
  const stop = subscribeCoreEvents({
    onEvent: (e) => {
      if (RELEVANT.has(e.type)) schedule();
    },
    // Back on screen: refresh what is loaded (the first load is the filter effect's).
    onResume: () => {
      if (lastLoad > 0) void loadFirst(current, items.length);
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

<div class="page">
  <search class="filters" aria-label={t("sessions-filters")}>
    <input
      bind:this={search}
      bind:value={searchText}
      oninput={onsearch}
      type="search"
      class="search"
      aria-label={t("sessions-search")}
      aria-keyshortcuts="Control+F"
      placeholder={t("sessions-search-placeholder")}
    />
    <Segmented
      value={current.range}
      options={rangeOptions}
      label={t("sessions-range")}
      onchange={setRange}
    />
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
    <Select
      value={current.project != null ? String(current.project) : ""}
      options={projectOptions}
      label={t("sessions-project")}
      onchange={(v: string) => apply({ project: v === "" ? null : Number(v) })}
    />
    <Select
      value={current.family ?? ""}
      options={familyOptions}
      label={t("sessions-model")}
      onchange={(v: string) => apply({ family: v === "" ? null : (v as ModelFamily) })}
    />
    <Toggle checked={current.live} label={t("sessions-live")} onchange={(live) => apply({ live })} />
    <Select
      value={current.sort}
      options={sortOptions}
      label={t("sessions-sort")}
      onchange={(sort: SessionSort) => apply({ sort })}
    />
  </search>

  {#if !loaded}
    <Skeleton width="100%" height="320px" />
  {:else if failed}
    <p class="error" role="alert">{t("main-load-failed")}</p>
  {:else if items.length === 0 && !loading}
    {#if filtered}
      <EmptyState title={t("sessions-empty-filtered")} body={t("sessions-empty-filtered-body")}>
        {#snippet action()}
          <a class="clear" href={hrefFor("sessions")}>{t("sessions-clear")}</a>
        {/snippet}
      </EmptyState>
    {:else}
      <EmptyState title={t("sessions-empty")} body={t("sessions-empty-body")} />
    {/if}
  {:else}
    <VirtualList
      bind:this={list}
      {items}
      key={(r) => r.id}
      label={t("sessions-list")}
      columns={COLUMNS}
      hasMore={cursor != null}
      {loading}
      onmore={loadMore}
      onactivate={open}
    >
      {#snippet header()}
        <span role="columnheader"><span class="visually-hidden">{t("sessions-running")}</span></span>
        <span role="columnheader">{t("sessions-col-name")}</span>
        <span role="columnheader">{t("sessions-col-start")}</span>
        <span role="columnheader" class="num">{t("sessions-col-duration")}</span>
        <span role="columnheader">{t("sessions-col-models")}</span>
        <span role="columnheader" class="num">{t("sessions-col-cost")}</span>
        <span role="columnheader" class="num">{t("sessions-col-tokens")}</span>
        <span role="columnheader" class="num">{t("sessions-col-impact")}</span>
        <span role="columnheader" class="num" title={t("sessions-col-subagents")}>↳</span>
      {/snippet}
      {#snippet row(r: SessionRow)}
        {@const impact = impactOf(r)}
        {@const duration = durationOf(r)}
        <span role="gridcell">
          {#if r.live}<span class="dot" title={t("sessions-running")}></span>
            <span class="visually-hidden">{t("sessions-running")}</span>{/if}
        </span>
        <span role="gridcell" class="name">
          <span class="title" class:fallback={r.name.fallback} title={r.name.text}>
            {middleEllipsis(r.name.text, 72)}
          </span>
          {#if r.transcript_deleted}
            <span title={t("sessions-deleted-tip")}><Badge tone="muted">{t("sessions-deleted")}</Badge></span>
          {/if}
          {#if r.project}<span class="project">{r.project.name}</span>{/if}
        </span>
        <span
          role="gridcell"
          class="muted"
          title={r.first_ts != null ? new Date(r.first_ts).toLocaleString(locale()) : undefined}
        >
          {ago(r.first_ts)}
        </span>
        <span role="gridcell" class="num readout">{duration != null ? dur(locale(), duration) : t("value-none")}</span>
        <span role="gridcell" class="models" title={r.models.map(modelLabel).join(", ")}>
          {#each r.models.slice(0, 2) as m (m)}<Badge>{modelLabel(m)}</Badge>{/each}
          {#if r.models.length > 2}<span class="more">+{r.models.length - 2}</span>{/if}
        </span>
        <span role="gridcell" class="num readout">{money(r.cost)}</span>
        <span role="gridcell" class="num readout">{tokensOf(r)}</span>
        <span
          role="gridcell"
          class="num readout"
          class:dim={impact?.dim}
          title={impact
            ? `${t(impact.kind === "weekly" ? "sessions-impact-weekly" : "sessions-impact-session")}${impact.dim ? ` ${t("sessions-impact-dim")}` : ""}`
            : undefined}
        >
          {impact
            ? t("value-pct", { pct: impact.percent, estimate: impact.kind === "weekly" || r.impact_basis?.approx ? "yes" : "no" })
            : t("value-none")}
        </span>
        <span role="gridcell" class="num readout muted">{r.subagents > 0 ? r.subagents : ""}</span>
      {/snippet}
    </VirtualList>
    {#if loading && items.length > 0}
      <p class="visually-hidden" role="status">{t("sessions-loading-more")}</p>
    {/if}
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
  .num {
    text-align: end;
    white-space: nowrap;
  }
  .muted {
    color: var(--text-2);
    font: var(--type-caption);
  }
  .dim {
    color: var(--text-3);
  }
  .dot {
    display: block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--signal-ok);
  }
  .name {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
    font: var(--type-body);
    color: var(--text-1);
  }
  .title {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .fallback {
    color: var(--text-2);
  }
  .project {
    flex: none;
    max-width: 40%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: var(--type-caption);
    color: var(--text-2);
  }
  .models {
    display: flex;
    gap: 4px;
    min-width: 0;
    overflow: hidden;
  }
  .more {
    font: var(--type-caption);
    color: var(--text-2);
  }
  .readout {
    font-size: 12px;
  }
  .clear {
    color: var(--text-1);
    font: var(--type-body);
  }
  .error {
    padding: var(--space-2) var(--space-3);
    border-left: 2px solid var(--signal-crit);
    background: var(--surface-1);
    border-radius: var(--radius-control);
  }
</style>

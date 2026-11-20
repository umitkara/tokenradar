<!--
  The windows page (Step 17a, ADR-070), weekly first:

  - Weekly cycles (default): one row per weekly window.
  - 5-hour windows: the day strip, offered only when the account has 5-hour windows at all
    (verified Ultra has none, ADR-041). Opened by URL without them it explains why it is empty.
  - List: every window of every kind.

  The range and the tab live in the URL fragment (`filter.ts`). A window opens in a drawer at
  `#windows/<id>` over the page. Refreshes while visible on new usage, at most every 10 s.
-->
<script lang="ts">
import { onMount, untrack } from "svelte";
import EmptyState from "../../../lib/components/EmptyState.svelte";
import Segmented from "../../../lib/components/Segmented.svelte";
import Skeleton from "../../../lib/components/Skeleton.svelte";
import { t } from "../../../lib/i18n/i18n.svelte";
import {
  type CoreEvent,
  commands,
  type WindowStrip as StripData,
  type WeekCycles as WeekData,
  type WindowRow,
} from "../../../lib/ipc/bindings";
import { subscribeCoreEvents } from "../../../lib/ipc/core-events";
import { inTauri } from "../../../lib/ipc/runtime";
import ExportButton from "../ExportButton.svelte";
import { hrefFor } from "../router";
import {
  parseWindowsQuery,
  spanRange,
  toWindowsQuery,
  WINDOWS_TABS,
  type WindowsQuery,
  type WindowsTab,
  windowsMemory,
} from "./filter";
import SpanFilter from "./SpanFilter.svelte";
import WeekCycles from "./WeekCycles.svelte";
import WindowList from "./WindowList.svelte";
import WindowStrip from "./WindowStrip.svelte";

export interface WindowsPreset {
  weeks: WeekData;
  strip: StripData;
  rows: WindowRow[];
}

interface Props {
  query: string;
  /** Synthetic data for the gallery; skips the backend. */
  preset?: WindowsPreset | null;
}

let { query, preset = null }: Props = $props();

const PAGE = 100;
const MIN_GAP_MS = 10_000;
const RELEVANT = new Set<CoreEvent["type"]>([
  "usage_appended",
  "calibration_updated",
  "limits_updated",
]);

let current = $state<WindowsQuery>(untrack(() => parseWindowsQuery(query)));
let weeks = $state<WeekData | null>(untrack(() => preset?.weeks) ?? null);
let strip = $state<StripData | null>(untrack(() => preset?.strip) ?? null);
let rows = $state<WindowRow[]>(untrack(() => preset?.rows) ?? []);
let cursor = $state<string | null>(null);
let loading = $state(false);
let failed = $state(false);
let fetched = $state(false);
let generation = 0;
let lastLoad = 0;
let pending: ReturnType<typeof setTimeout> | null = null;

$effect(() => {
  windowsMemory.query = query;
  current = parseWindowsQuery(query);
});

$effect(() => {
  const q = current;
  if (preset || !inTauri()) return;
  void load(q);
});

async function load(q: WindowsQuery) {
  const mine = ++generation;
  loading = true;
  lastLoad = Date.now();
  const range = spanRange(q, new Date());
  // The strip says whether the account has 5-hour windows, which decides the tabs.
  const [s, second] = await Promise.all([
    commands.windowStrip(range),
    q.tab === "list"
      ? commands.listWindows({ range, limit: null }, null, PAGE)
      : q.tab === "weeks"
        ? commands.listWeeks(range)
        : Promise.resolve(null),
  ]);
  if (mine !== generation) return;
  loading = false;
  fetched = true;
  let ok = s.status === "ok";
  if (s.status === "ok") strip = s.data;
  if (second && second.status === "ok") {
    if ("items" in second.data && "next" in second.data) {
      rows = second.data.items;
      cursor = second.data.next;
    } else {
      weeks = second.data as WeekData;
    }
  } else if (second) {
    ok = false;
  }
  failed = !ok;
}

async function loadMore() {
  if (!cursor || loading) return;
  const mine = generation;
  loading = true;
  const result = await commands.listWindows(
    { range: spanRange(current, new Date()), limit: null },
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
    void load(current);
  }, wait);
}

function apply(change: Partial<WindowsQuery>) {
  location.hash = hrefFor("windows", null, toWindowsQuery({ ...current, ...change }));
}

const tabs = $derived(
  WINDOWS_TABS.filter(
    (tab) => tab !== "strip" || strip?.any === true || current.tab === "strip",
  ).map((value) => ({
    value,
    label: t(`windows-tab-${value}`),
  })),
);
const text = $derived(toWindowsQuery(current));

onMount(() => {
  if (preset) return;
  const stop = subscribeCoreEvents({
    onEvent: (e) => {
      if (RELEVANT.has(e.type)) schedule();
    },
    onResume: () => {
      if (lastLoad > 0) void load(current);
    },
  });
  return () => {
    stop();
    if (pending) clearTimeout(pending);
  };
});
</script>

<!-- The strip and the list scroll inside the page; the weekly table grows with its rows. -->
<div class="page" class:fill={current.tab !== "weeks"}>
  <search class="filters" aria-label={t("windows-filters")}>
    <Segmented value={current.tab} options={tabs} label={t("windows-tabs")} onchange={(tab: WindowsTab) => apply({ tab })} />
    <SpanFilter value={current} onchange={apply} />
    <span class="grow"></span>
    <ExportButton kind="windows" request={() => ({ kind: "windows", range: spanRange(current, new Date()) })} />
  </search>

  {#if failed}
    <p class="error" role="alert">{t("main-load-failed")}</p>
  {:else if current.tab === "weeks"}
    {#if !weeks}
      <Skeleton width="100%" height="320px" />
    {:else if weeks.items.length === 0}
      <EmptyState title={t("windows-empty")} />
    {:else}
      <WeekCycles data={weeks} query={text} />
    {/if}
  {:else if current.tab === "strip"}
    {#if !strip}
      <Skeleton width="100%" height="320px" />
    {:else if !strip.any}
      <EmptyState title={t("windows-strip-none-title")} body={t("windows-strip-none-body")}>
        {#snippet action()}
          <a href={hrefFor("windows", null, toWindowsQuery({ ...current, tab: "weeks" }))}>{t("windows-tab-weeks")}</a>
        {/snippet}
      </EmptyState>
    {:else if strip.days.length === 0}
      <EmptyState title={t("windows-empty")} />
    {:else}
      <WindowStrip data={strip} query={text} />
    {/if}
  {:else if rows.length === 0 && !loading && (preset || fetched)}
    <EmptyState title={t("windows-empty")} />
  {:else}
    <WindowList {rows} hasMore={cursor != null} {loading} onmore={loadMore} query={text} />
  {/if}
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .page.fill {
    flex: 1 1 0;
    min-height: 0;
  }
  .filters {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-3);
  }
  .grow {
    flex: 1 1 0;
  }
  .error {
    color: var(--text-1);
    border-left: 2px solid var(--signal-crit);
    padding-left: var(--space-2);
  }
</style>

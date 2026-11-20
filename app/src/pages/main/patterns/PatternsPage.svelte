<!--
  The patterns page (Step 17a, ADR-070): when the usage happens and how it changes.

  - Week day × hour heat map (cost, tokens or `≈` % of the weekly limit), in the locale's week
    order, with the time-of-day bars (weekday and weekend averages) from the same totals.
  - Limit risk: start hours of 5-hour windows that hit the limit; only with 5-hour windows.
  - Model mix by week, weeks starting on the locale's first day.

  Range and metric live in the URL fragment. Refreshes while visible on new usage, at most
  every 10 s.
-->
<script lang="ts">
import { onMount, untrack } from "svelte";
import Card from "../../../lib/components/Card.svelte";
import EmptyState from "../../../lib/components/EmptyState.svelte";
import Segmented from "../../../lib/components/Segmented.svelte";
import Skeleton from "../../../lib/components/Skeleton.svelte";
import { locale, t } from "../../../lib/i18n/i18n.svelte";
import {
  type CoreEvent,
  commands,
  type HeatMetric,
  type Heatmap,
  type LimitRisk,
  type MixTrend,
} from "../../../lib/ipc/bindings";
import { subscribeCoreEvents } from "../../../lib/ipc/core-events";
import { inTauri } from "../../../lib/ipc/runtime";
import { usd } from "../../panel/detail";
import ExportButton from "../ExportButton.svelte";
import { countOf } from "../overview/view";
import { hrefFor } from "../router";
import {
  HEAT_METRICS,
  type PatternsQuery,
  parsePatternsQuery,
  spanRange,
  toPatternsQuery,
} from "../windows/filter";
import SpanFilter from "../windows/SpanFilter.svelte";
import { firstWeekday } from "../windows/view";
import HeatGrid from "./HeatGrid.svelte";
import HourBars from "./HourBars.svelte";
import MixChart from "./MixChart.svelte";
import RiskCard from "./RiskCard.svelte";

export interface PatternsPreset {
  heatmap: Heatmap;
  risk: LimitRisk;
  mix: MixTrend;
}

interface Props {
  query: string;
  /** Synthetic data for the gallery; skips the backend. */
  preset?: PatternsPreset | null;
}

let { query, preset = null }: Props = $props();

const MIN_GAP_MS = 10_000;
const RELEVANT = new Set<CoreEvent["type"]>(["usage_appended", "calibration_updated"]);

let current = $state<PatternsQuery>(untrack(() => parsePatternsQuery(query)));
let heatmap = $state<Heatmap | null>(untrack(() => preset?.heatmap) ?? null);
let risk = $state<LimitRisk | null>(untrack(() => preset?.risk) ?? null);
let mix = $state<MixTrend | null>(untrack(() => preset?.mix) ?? null);
let failed = $state(false);
let generation = 0;
let lastLoad = 0;
let pending: ReturnType<typeof setTimeout> | null = null;

const first = $derived(
  firstWeekday(locale(), typeof navigator === "undefined" ? undefined : navigator.language),
);

$effect(() => {
  current = parsePatternsQuery(query);
});

$effect(() => {
  const q = current;
  const ws = first;
  if (preset || !inTauri()) return;
  void load(q, ws);
});

async function load(q: PatternsQuery, weekStart: number) {
  const mine = ++generation;
  lastLoad = Date.now();
  const range = spanRange(q, new Date());
  const [h, r, m] = await Promise.all([
    commands.getHeatmap(range, q.metric),
    commands.limitRisk(range),
    commands.modelMixTrend(range, weekStart),
  ]);
  if (mine !== generation) return;
  if (h.status === "ok") heatmap = h.data;
  if (r.status === "ok") risk = r.data;
  if (m.status === "ok") mix = m.data;
  failed = h.status === "error" && r.status === "error" && m.status === "error";
}

function schedule() {
  if (pending || preset) return;
  const wait = Math.max(0, lastLoad + MIN_GAP_MS - Date.now());
  pending = setTimeout(() => {
    pending = null;
    void load(current, first);
  }, wait);
}

function apply(change: Partial<PatternsQuery>) {
  location.hash = hrefFor("patterns", null, toPatternsQuery({ ...current, ...change }));
}

function format(metric: HeatMetric): (v: number) => string {
  switch (metric) {
    case "percent":
      return (v) => t("value-pct", { pct: v, estimate: "yes" });
    case "cost":
      return (v) => t("sessions-usd", { usd: usd(v) });
    case "tokens":
      return (v) => t("overview-count", countOf(v));
  }
}

const metrics = $derived(
  HEAT_METRICS.map((value) => ({ value, label: t(`patterns-metric-${value}`) })),
);
const noPercent = $derived(heatmap?.metric === "percent" && heatmap.basis == null);
const emptyHeat = $derived(heatmap != null && (heatmap.max_total ?? 0) <= 0);

onMount(() => {
  if (preset) return;
  const stop = subscribeCoreEvents({
    onEvent: (e) => {
      if (RELEVANT.has(e.type)) schedule();
    },
    onResume: () => {
      if (lastLoad > 0) void load(current, first);
    },
  });
  return () => {
    stop();
    if (pending) clearTimeout(pending);
  };
});
</script>

<div class="page">
  <search class="filters" aria-label={t("patterns-filters")}>
    <SpanFilter value={current} onchange={apply} />
    <span class="grow"></span>
    <ExportButton kind="daily" request={() => ({ kind: "daily", range: spanRange(current, new Date()) })} />
  </search>

  {#if failed}
    <p class="error" role="alert">{t("main-load-failed")}</p>
  {/if}

  <Card title={t("patterns-heatmap")}>
    {#snippet corner()}
      <Segmented
        value={current.metric}
        options={metrics}
        label={t("patterns-metric")}
        onchange={(metric: HeatMetric) => apply({ metric })}
      />
    {/snippet}
    {#if !heatmap}
      <Skeleton width="100%" height="200px" />
    {:else if noPercent}
      <p class="note">{t("patterns-percent-none")}</p>
    {:else if emptyHeat}
      <EmptyState title={t("patterns-empty")} />
    {:else}
      <HeatGrid cells={heatmap.cells} max={heatmap.max_total ?? 0} {first} format={format(heatmap.metric)} />
    {/if}
  </Card>

  <div class="pair">
    <Card title={t("patterns-hours")}>
      {#if !heatmap}
        <Skeleton width="100%" height="160px" />
      {:else if noPercent || emptyHeat}
        <p class="note">{t(noPercent ? "patterns-percent-none" : "patterns-empty")}</p>
      {:else}
        <HourBars hours={heatmap.hours} format={format(heatmap.metric)} />
      {/if}
    </Card>

    <Card title={t("patterns-risk")}>
      {#if !risk}
        <Skeleton width="100%" height="160px" />
      {:else}
        <RiskCard {risk} />
      {/if}
    </Card>
  </div>

  <Card title={t("patterns-mix")}>
    {#if !mix}
      <Skeleton width="100%" height="160px" />
    {:else if mix.weeks.length === 0 || mix.weeks.every((w) => w.total <= 0)}
      <EmptyState title={t("patterns-empty")} />
    {:else}
      <MixChart trend={mix} />
    {/if}
  </Card>
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
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
  .pair {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
    gap: var(--space-3);
  }
  .note {
    margin: 0;
    font: var(--type-caption);
    color: var(--text-2);
  }
  .error {
    color: var(--text-1);
    border-left: 2px solid var(--signal-crit);
    padding-left: var(--space-2);
  }
</style>

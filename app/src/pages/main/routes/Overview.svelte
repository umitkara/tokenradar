<!--
  The overview (Step 15). For now the 7-day chart of the 5-hour and weekly limits, which shows
  the chart wrapper on real data; the page's sections follow.
-->
<script lang="ts">
import { onMount } from "svelte";
import Chart from "../../../lib/chart/Chart.svelte";
import Card from "../../../lib/components/Card.svelte";
import EmptyState from "../../../lib/components/EmptyState.svelte";
import Skeleton from "../../../lib/components/Skeleton.svelte";
import { t } from "../../../lib/i18n/i18n.svelte";
import { commands, type Overview } from "../../../lib/ipc/bindings";
import { inTauri } from "../../../lib/ipc/runtime";
import { chartOf } from "../series";

let _props: { param: string | null } = $props();

let overview = $state<Overview | null>(null);
let failed = $state(false);
let loaded = $state(!inTauri());

const DAY = 86_400_000;
const chart = $derived.by(() => {
  if (!overview) return null;
  const to = overview.generated_at;
  return chartOf(overview.series, to - 7 * DAY, to);
});

async function load() {
  const result = await commands.getOverview();
  if (result.status === "ok") {
    overview = result.data;
    failed = false;
  } else {
    failed = true;
  }
  loaded = true;
}

onMount(() => {
  if (inTauri()) void load();
});
</script>

<EmptyState title={t("main-soon-overview")} body={t("main-soon-overview-body")} />
<Card title={t("main-chart-7d")}>
  {#if !loaded}
    <Skeleton width="100%" height="220px" />
  {:else if failed}
    <p class="note">{t("main-load-failed")}</p>
  {:else if chart}
    <Chart model={chart} height={220} />
  {:else}
    <p class="note">{t("chart-empty")}</p>
  {/if}
</Card>

<style>
  .note {
    margin: 0;
    color: var(--text-2);
  }
</style>

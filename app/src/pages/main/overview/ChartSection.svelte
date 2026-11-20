<!--
  The last 7 days (Step 15b): the 5-hour and weekly percentages with window bands, hit-limit
  windows in red and estimates dashed, and under it the lane of the sessions with the largest
  share of the week.
-->
<script lang="ts">
import Chart from "../../../lib/chart/Chart.svelte";
import Card from "../../../lib/components/Card.svelte";
import { t } from "../../../lib/i18n/i18n.svelte";
import type { Overview } from "../../../lib/ipc/bindings";
import { chartOf } from "../series";
import { spansOf } from "./view";

let { overview }: { overview: Overview } = $props();

const DAY = 86_400_000;
const model = $derived.by(() => {
  const to = overview.generated_at;
  return { ...chartOf(overview.series, to - 7 * DAY, to), spans: spansOf(overview.series) };
});
</script>

<Card title={t("main-chart-7d")}>
  <Chart {model} height={220} />
  {#if model.spans.length > 0}
    <p class="note">{t("overview-spans-note")}</p>
  {/if}
</Card>

<style>
  .note {
    margin: var(--space-2) 0 0;
    font: var(--type-caption);
    color: var(--text-2);
  }
</style>

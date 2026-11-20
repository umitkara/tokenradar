<!--
  The overview's sections, top to bottom (design-language.md §5, ADR-067). Shared by the page
  and the gallery, so the gallery shows the real layout with synthetic data.
-->
<script lang="ts">
import type { Overview } from "../../../lib/ipc/bindings";
import ChartSection from "./ChartSection.svelte";
import NowSection from "./NowSection.svelte";
import OtherSection from "./OtherSection.svelte";
import PlanSection from "./PlanSection.svelte";
import TopSection from "./TopSection.svelte";
import WeekSection from "./WeekSection.svelte";

let { overview }: { overview: Overview } = $props();
</script>

<NowSection {overview} />
<ChartSection {overview} />
<WeekSection {overview} />
<TopSection {overview} />
<div class="pair">
  <OtherSection {overview} />
  <PlanSection plan={overview.plan} />
</div>

<style>
  .pair {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(320px, 100%), 1fr));
    gap: var(--space-4);
    align-items: start;
  }
</style>

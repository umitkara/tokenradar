<!--
  The overview (Step 15b). One page that answers, top to bottom: where am I now, how is this week
  going, what is using it, how much value am I getting (design-language.md §5). Everything comes
  from `get_overview`; the page lays it out.

  - Refreshes from core events while visible (usage, limits, calibration), at most every 10 s,
    and in full whenever the window becomes visible again (`subscribeCoreEvents`).
-->
<script lang="ts">
import { onMount, tick } from "svelte";
import Card from "../../../lib/components/Card.svelte";
import Skeleton from "../../../lib/components/Skeleton.svelte";
import { t } from "../../../lib/i18n/i18n.svelte";
import { type CoreEvent, commands, type Overview } from "../../../lib/ipc/bindings";
import { subscribeCoreEvents } from "../../../lib/ipc/core-events";
import { inTauri } from "../../../lib/ipc/runtime";
import OverviewBody from "../overview/OverviewBody.svelte";

let _props: { param: string | null } = $props();

let overview = $state<Overview | null>(null);
let failed = $state(false);
let loaded = $state(!inTauri());

/** Events that change what the page shows. */
const RELEVANT = new Set<CoreEvent["type"]>([
  "usage_appended",
  "limits_updated",
  "calibration_updated",
  "calibration_suspect",
]);
const MIN_GAP_MS = 10_000;
let lastLoad = 0;
let pending: ReturnType<typeof setTimeout> | null = null;
let loading = false;

async function load() {
  if (loading) return;
  loading = true;
  lastLoad = Date.now();
  const result = await commands.getOverview();
  loading = false;
  if (result.status === "ok") {
    const first = overview == null;
    overview = result.data;
    failed = false;
    if (first) void markPainted();
  } else if (!overview) {
    failed = true;
  }
  loaded = true;
}

/** `overview:data`: from navigation to the first paint with data (perf budget, Step 15b). */
async function markPainted() {
  await tick();
  requestAnimationFrame(() => {
    if (performance.getEntriesByName("overview:data").length > 0) return;
    performance.measure("overview:data", { start: 0, end: performance.now() });
  });
}

function schedule() {
  if (pending) return;
  const wait = Math.max(0, lastLoad + MIN_GAP_MS - Date.now());
  pending = setTimeout(() => {
    pending = null;
    void load();
  }, wait);
}

onMount(() => {
  const stop = subscribeCoreEvents({
    onEvent: (e) => {
      if (RELEVANT.has(e.type)) schedule();
    },
    onResume: () => void load(),
  });
  return () => {
    stop();
    if (pending) clearTimeout(pending);
  };
});
</script>

<div class="overview">
  {#if !loaded}
    <Card><Skeleton width="100%" height="168px" /></Card>
    <Card><Skeleton width="100%" height="220px" /></Card>
  {:else if failed || !overview}
    <p class="error" role="alert">{t("main-load-failed")}</p>
  {:else}
    <OverviewBody {overview} />
  {/if}
</div>

<style>
  .overview {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    container-type: inline-size;
    min-width: 0;
  }
  .error {
    padding: var(--space-2) var(--space-3);
    border-left: 2px solid var(--signal-crit);
    background: var(--surface-1);
    border-radius: var(--radius-control);
  }
</style>

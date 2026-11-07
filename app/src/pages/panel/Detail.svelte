<!--
  The panel's detail view (ADR-048): the Claude Code mod's report (ADR-046) inside the app.
  Limit cards with their own source, reset, projection and pace; the time strip and the usage
  chart of one limit (the weekly one, else the 5-hour window); the sessions with their cost.
  Loaded on demand, so the summary's cold start does not carry it. While shown it fetches the
  detail again every minute: the pace moves with the clock. No timer runs while hidden.
-->
<script lang="ts">
import Card from "../../lib/components/Card.svelte";
import { limitName } from "../../lib/components/limit";
import Skeleton from "../../lib/components/Skeleton.svelte";
import { weekdayClock } from "../../lib/i18n/format";
import { hour12, locale, t } from "../../lib/i18n/i18n.svelte";
import type { PanelDetail, PanelSnapshot } from "../../lib/ipc/bindings";
import DetailLimits from "./DetailLimits.svelte";
import SessionsCard from "./SessionsCard.svelte";
import { DETAIL_REFRESH_MS, loadDetail } from "./state.svelte";
import TimeStrip from "./TimeStrip.svelte";
import UsageChart from "./UsageChart.svelte";
import { emptyState } from "./view";

interface Props {
  snapshot: PanelSnapshot;
  detail: PanelDetail | null;
}

let { snapshot, detail }: Props = $props();

const focus = $derived(detail?.focus ?? null);
const focusLimit = $derived(
  focus ? (snapshot.limits.find((l) => l.kind === focus.kind && l.scope == null) ?? null) : null,
);
const focusPace = $derived(
  focus ? (detail?.pace.find((p) => p.kind === focus.kind && p.scope == null) ?? null) : null,
);
const weekly = $derived((focus?.periods.length ?? 0) > 0);
const resetText = $derived.by(() => {
  if (!focus) return "";
  const d = new Date(focus.resets_at);
  return weekdayClock(locale(), d.getDay(), d.getHours(), d.getMinutes(), hour12());
});
const anyEstimate = $derived(
  snapshot.limits.some((l) => l.source === "estimate") ||
    snapshot.live_sessions.some((s) => s.share_percent != null) ||
    snapshot.top_sessions.some((s) => s.share_percent != null),
);

$effect(() => {
  const timer = setInterval(() => {
    if (document.visibilityState === "visible") void loadDetail(true);
  }, DETAIL_REFRESH_MS);
  return () => clearInterval(timer);
});
</script>

<DetailLimits {snapshot} {detail} />

{#if detail == null}
  <div class="loading" aria-busy="true">
    <span class="visually-hidden">{t("detail-loading")}</span>
    <Skeleton width="100%" height="96px" />
  </div>
{:else if focus && focusLimit}
  <Card
    title={weekly
      ? t("detail-strip-title", {
          name: limitName(focusLimit),
          elapsed: Math.round(focusPace?.elapsed_percent ?? 0),
        })
      : t("detail-chart-title", { name: limitName(focusLimit) })}
  >
    <div class="report">
      {#if weekly}<TimeStrip {focus} limit={focusLimit} />{/if}
      <UsageChart {focus} limit={focusLimit} pace={focusPace} />
      <p class="legend">
        <span>{t("resets-at", { at: resetText })}</span>
        {#if weekly}
          <span><i class="key midnight" aria-hidden="true"></i>{t("detail-legend-midnight")}</span>
          <span><i class="key period" aria-hidden="true"></i>{t("detail-legend-period")}</span>
        {/if}
        <span><i class="key time" aria-hidden="true"></i>{t("detail-legend-time")}</span>
        <span><i class="key use {focusLimit.severity}" aria-hidden="true"></i>{t("detail-legend-use")}</span>
      </p>
    </div>
  </Card>
{/if}

{#if emptyState(snapshot) !== "no-claude"}
  <section aria-label={t("panel-sessions")}>
    <SessionsCard {snapshot} detail />
  </section>
{/if}

{#if anyEstimate}
  <p class="note">{t("detail-estimate-note")}</p>
{/if}

<style>
  .report {
    display: grid;
    gap: var(--space-2);
  }
  .legend {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    column-gap: var(--space-3);
    row-gap: 2px;
    font: var(--type-caption);
    color: var(--text-2);
  }
  .legend span {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }
  .key {
    display: inline-block;
    width: 12px;
    height: 10px;
    flex: none;
  }
  .key.midnight {
    width: 0;
    border-left: 1px dotted var(--text-2);
  }
  .key.period {
    width: 1px;
    background: var(--text-2);
  }
  .key.time {
    height: 0;
    border-top: 1.5px dashed var(--text-3);
  }
  .key.use {
    border-radius: 2px;
    background: var(--signal-ok);
    opacity: 0.6;
  }
  .key.use.warn {
    background: var(--signal-warn);
  }
  .key.use.crit {
    background: var(--signal-crit);
  }
  .note {
    margin: 0;
    font: var(--type-caption);
    color: var(--text-3);
  }
  @media (forced-colors: active) {
    .key.period,
    .key.use {
      forced-color-adjust: none;
      background: CanvasText;
    }
  }
</style>

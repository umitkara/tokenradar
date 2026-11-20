<!--
  "Where am I?" (Step 15b): the panel's ring and limit rows at full size, the one projection the
  panel states, and where the numbers come from (source badge, calibration confidence). Without a
  reading it says what is missing and how to get it, never a made-up percentage.
-->
<script lang="ts">
import Button from "../../../lib/components/Button.svelte";
import Card from "../../../lib/components/Card.svelte";
import EmptyState from "../../../lib/components/EmptyState.svelte";
import LimitBar from "../../../lib/components/LimitBar.svelte";
import RingGauge from "../../../lib/components/RingGauge.svelte";
import SourceBadge from "../../../lib/components/SourceBadge.svelte";
import { t } from "../../../lib/i18n/i18n.svelte";
import { commands, type Overview, type SettingsSection } from "../../../lib/ipc/bindings";
import { inTauri } from "../../../lib/ipc/runtime";
import { clock } from "../../../lib/time/clock.svelte";
import { limitsOf, projectionLine } from "../../panel/view";
import { dataState } from "./view";

let { overview }: { overview: Overview } = $props();

const limits = $derived(limitsOf(overview));
const state = $derived(dataState(overview));
const primary = $derived(limits.fiveHour ?? limits.weekly);
const projection = $derived(projectionLine(limits));
const projectionText = $derived.by(() => {
  if (!projection) return "";
  if (projection.type === "idle") return t("panel-projection-idle");
  if (projection.type === "at-reset") {
    return t("panel-projection-at-reset", { kind: projection.kind, pct: projection.pct });
  }
  return t("panel-projection-will-hit", {
    kind: projection.kind,
    approx: projection.low ? "yes" : "no",
    eta: Math.max(0, projection.etaAt - clock.now),
    reset: Math.max(0, projection.resetAt - clock.now),
  });
});
const calibration = $derived(overview.calibration);

function open(section: SettingsSection) {
  if (inTauri()) void commands.openSettings(section);
}
</script>

<Card title={t("overview-now")}>
  {#snippet corner()}
    {#if primary}
      <!-- At the card's end: the tooltip anchors to the badge's end so it stays inside. -->
      <span class="tip-end">
        <SourceBadge
          source={primary.source}
          observedAt={primary.observed_at}
          combinedWith={primary.combined_with}
        />
      </span>
    {/if}
  {/snippet}
  {#if state.noLimits}
    <EmptyState title={t("panel-no-limits")} body={t("panel-estimate-mode")}>
      {#snippet action()}
        <div class="ctas">
          <Button variant="primary" onclick={() => open("statusline")}>
            {t("panel-install-statusline")}
          </Button>
          <Button onclick={() => open("api")}>{t("panel-allow-api")}</Button>
        </div>
      {/snippet}
    </EmptyState>
  {:else}
    <div class="now">
      <RingGauge outer={limits.weekly} inner={limits.fiveHour} single={limits.fiveHourAbsent} />
      <div class="side">
        <div class="rows">
          {#if limits.fiveHour}<LimitBar limit={limits.fiveHour} />{/if}
          {#if limits.weekly}<LimitBar limit={limits.weekly} countdown="absolute" />{/if}
          {#each limits.scoped as scoped (scoped.scope?.model ?? scoped.scope?.surface)}
            <LimitBar limit={scoped} countdown="absolute" />
          {/each}
        </div>
        {#if projection}
          <p
            class="projection"
            class:crit={projection.type === "will-hit"}
            class:low={projection.type !== "idle" && projection.low}
            title={projection.type !== "idle" && projection.low
              ? t("panel-projection-low")
              : undefined}
          >
            {projectionText}
          </p>
        {/if}
        <p class="meta">
          {#if calibration}
            <span title={t("overview-calibration-tip", { n: calibration.n_obs })}>
              {t("overview-calibration", { level: calibration.level })}
            </span>
          {:else}
            <span>{t("overview-calibration-none")}</span>
          {/if}
        </p>
        {#if limits.fiveHourAbsent}
          <p class="note">{t("panel-no-five-hour")}</p>
        {/if}
        {#if state.estimatesOnly}
          <p class="note">{t("overview-estimates-only")}</p>
        {/if}
      </div>
    </div>
  {/if}
</Card>

<style>
  .now {
    display: flex;
    align-items: center;
    gap: var(--space-5);
  }
  .side {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .rows {
    display: grid;
    gap: var(--space-3);
  }
  .projection {
    margin: 0;
    font: var(--type-body);
    color: var(--text-1);
  }
  .projection.low {
    color: var(--text-2);
  }
  .meta,
  .note {
    margin: 0;
    font: var(--type-caption);
    color: var(--text-2);
  }
  .ctas {
    display: flex;
    gap: var(--space-2);
    flex-wrap: wrap;
    justify-content: center;
  }
</style>

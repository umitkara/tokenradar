<!--
  The ring and the limit rows (design-language.md §4): outer ring weekly, inner ring the 5-hour
  window, the rows beside it, then the one projection the panel states. Without honest data it
  shows what is missing and how to get it, never a made-up percentage (§9).
-->
<script lang="ts">
import Button from "../../lib/components/Button.svelte";
import Card from "../../lib/components/Card.svelte";
import EmptyState from "../../lib/components/EmptyState.svelte";
import LimitBar from "../../lib/components/LimitBar.svelte";
import RingGauge from "../../lib/components/RingGauge.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import type { PanelSnapshot, SettingsSection } from "../../lib/ipc/bindings";
import { clock } from "../../lib/time/clock.svelte";
import Icon from "./Icon.svelte";
import { emptyState, limitsOf, projectionLine } from "./view";

interface Props {
  snapshot: PanelSnapshot;
  openSettings: (section: SettingsSection) => void;
}

let { snapshot, openSettings }: Props = $props();

const limits = $derived(limitsOf(snapshot));
const empty = $derived(emptyState(snapshot));
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
const needsStatusline = $derived(snapshot.statusline.last_seen == null);
const needsApi = $derived(snapshot.api.state.state === "off");
</script>

<Card>
  <h2 class="visually-hidden">{t("panel-limits")}</h2>
  {#if empty === "no-claude"}
    <EmptyState title={t("panel-no-claude")} body={t("panel-no-claude-hint")}>
      {#snippet action()}
        <Button variant="primary" onclick={() => openSettings("accounts")}>
          {t("panel-choose-folder")}
        </Button>
      {/snippet}
    </EmptyState>
  {:else if empty === "no-limits"}
    <EmptyState
      title={t("panel-no-limits")}
      body={needsStatusline || needsApi ? t("panel-estimate-mode") : undefined}
    >
      {#snippet action()}
        <div class="ctas">
          {#if needsStatusline}
            <Button variant="primary" onclick={() => openSettings("statusline")}>
              {t("panel-install-statusline")}
            </Button>
          {/if}
          {#if needsApi}
            <Button onclick={() => openSettings("api")}>{t("panel-allow-api")}</Button>
          {/if}
        </div>
      {/snippet}
    </EmptyState>
  {:else}
    <div class="limits">
      <RingGauge outer={limits.weekly} inner={limits.fiveHour} size={120} />
      <div class="rows">
        {#if limits.fiveHour}<LimitBar limit={limits.fiveHour} layout="stacked" />{/if}
        {#if limits.weekly}
          <LimitBar limit={limits.weekly} countdown="absolute" layout="stacked" />
        {/if}
        {#each limits.scoped as scoped (scoped.scope?.model ?? scoped.scope?.surface)}
          <LimitBar limit={scoped} countdown="absolute" layout="stacked" />
        {/each}
      </div>
    </div>
    {#if projection}
      <p
        class="projection"
        class:crit={projection.type === "will-hit"}
        class:low={projection.type !== "idle" && projection.low}
        title={projection.type !== "idle" && projection.low ? t("panel-projection-low") : undefined}
      >
        {projectionText}
      </p>
    {/if}
    {#if snapshot.account?.extra_usage_enabled}
      <p class="extra"><Icon name="info" />{t("panel-extra-usage")}</p>
    {/if}
  {/if}
</Card>

<style>
  .limits {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .rows {
    flex: 1;
    min-width: 0;
    display: grid;
    gap: var(--space-2);
  }
  .projection,
  .extra {
    margin: var(--space-3) 0 0;
    font: var(--type-caption);
    color: var(--text-2);
  }
  .projection.crit {
    color: var(--signal-crit);
    font-weight: 600;
  }
  .projection.low {
    color: var(--text-3);
    font-weight: 400;
  }
  .extra {
    display: flex;
    gap: var(--space-1);
    align-items: center;
    margin-top: var(--space-2);
  }
  .ctas {
    display: grid;
    gap: var(--space-2);
    justify-items: start;
  }
  @media (forced-colors: active) {
    .projection.crit {
      color: CanvasText;
    }
  }
</style>

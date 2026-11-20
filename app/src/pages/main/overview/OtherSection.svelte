<!--
  "Other" (Step 15b): weekly-limit use that is not Claude Code on this computer. Our own estimate
  (from the calibration, ADR-064) next to Anthropic's `seven_day_breakdown`, with a note when the
  two disagree by more than 10 points. Our estimate is only computed with a usable calibration.
-->
<script lang="ts">
import Card from "../../../lib/components/Card.svelte";
import { t } from "../../../lib/i18n/i18n.svelte";
import type { Overview } from "../../../lib/ipc/bindings";
import { surfaceKey } from "./view";

let { overview }: { overview: Overview } = $props();

const ours = $derived(overview.other.other);
const rows = $derived(
  overview.other.breakdown.map(([key, percent]) => {
    const k = surfaceKey(key);
    return { key, label: k ? t(k) : key, percent };
  }),
);
</script>

<Card title={t("overview-other")}>
  <p class="body">{t("overview-other-body")}</p>
  <dl>
    <dt>{t("overview-other-ours")}</dt>
    <dd>
      {#if ours?.percent != null}
        <span class="readout">{t("value-pct", { pct: ours.percent, estimate: "yes" })}</span>
        {#if ours.share_of_total != null}
          <span class="sub">{t("overview-other-ours-share", { pct: ours.share_of_total })}</span>
        {/if}
      {:else}
        <span class="sub">{t("overview-other-none")}</span>
      {/if}
    </dd>
    <dt>{t("overview-other-api")}</dt>
    <dd>
      {#if rows.length > 0}
        <ul>
          {#each rows as row (row.key)}
            <li>
              <span>{row.label}</span>
              <span class="readout">{row.percent == null ? t("value-none") : t("value-pct", { pct: row.percent, estimate: "no" })}</span>
            </li>
          {/each}
        </ul>
      {:else}
        <span class="sub">{t("overview-other-no-breakdown")}</span>
      {/if}
    </dd>
  </dl>
  {#if ours?.suspect}
    <p class="info" role="note">{t("overview-other-suspect")}</p>
  {:else if ours?.other_excess}
    <p class="info" role="note">{t("overview-other-excess")}</p>
  {/if}
</Card>

<style>
  .body {
    margin: 0 0 var(--space-3);
    font: var(--type-caption);
    color: var(--text-2);
  }
  dl {
    margin: 0;
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--space-2) var(--space-4);
  }
  dt {
    font: var(--type-body);
    color: var(--text-2);
  }
  dd {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--space-2);
    color: var(--text-1);
  }
  .sub {
    font: var(--type-caption);
    color: var(--text-2);
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    grid-template-columns: max-content max-content;
    gap: 2px var(--space-3);
  }
  li {
    display: contents;
  }
  .info {
    margin: var(--space-3) 0 0;
    padding: var(--space-2) var(--space-3);
    border-left: 2px solid var(--signal-other);
    background: var(--surface-2);
    border-radius: var(--radius-control);
    font: var(--type-caption);
    color: var(--text-1);
  }
</style>

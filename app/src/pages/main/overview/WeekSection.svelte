<!--
  "How is this week going?" (Step 15b): the week's API-equivalent cost against the same point
  last week, then what it was made of: tokens by kind and the model mix, as magnitude bars (no
  donut). The week is the open weekly window, or the last 7 days without one.
-->
<script lang="ts">
import Card from "../../../lib/components/Card.svelte";
import { pct, weekdayClock } from "../../../lib/i18n/format";
import { hour12, locale, t } from "../../../lib/i18n/i18n.svelte";
import type { Overview } from "../../../lib/ipc/bindings";
import BarList, { type BarItem } from "./BarList.svelte";
import { changeOf, countOf, modelRows, tokenRows, usd } from "./view";

let { overview }: { overview: Overview } = $props();

const TOKEN_KEYS = {
  input: "overview-token-input",
  output: "overview-token-output",
  cache_read: "overview-token-cache-read",
  cache_write: "overview-token-cache-write",
} as const;

const since = $derived.by(() => {
  if (overview.week.window_id == null) return t("overview-week-rolling");
  const d = new Date(overview.week.starts_at);
  const at = weekdayClock(locale(), d.getDay(), d.getHours(), d.getMinutes(), hour12());
  return t("overview-week-since", { at });
});
const change = $derived(changeOf(overview.change));
const count = (n: number) => t("overview-count", countOf(n));
const tokenTotal = $derived(tokenRows(overview.tokens).reduce((s, r) => s + r.value, 0));
const tokens = $derived<BarItem[]>(
  tokenRows(overview.tokens).map((r) => ({
    key: r.key,
    label: t(TOKEN_KEYS[r.key]),
    width: r.width,
    value: count(r.value),
    title: `${count(r.value)} · ${pct(locale(), r.share)}`,
  })),
);
const models = $derived<BarItem[]>(
  modelRows(overview.models).map((r) => ({
    key: r.key || "__rest",
    label: r.key || t("overview-model-rest"),
    width: r.width,
    value: t("overview-cost-value", { usd: usd(r.value) }),
    title: pct(locale(), r.share),
  })),
);
const empty = $derived(overview.cost === 0 && tokenTotal === 0);
</script>

<Card title={t("overview-week")}>
  {#snippet corner()}
    <span class="since">{since}</span>
  {/snippet}
  {#if empty}
    <p class="note">{t("overview-week-empty")}</p>
  {:else}
    <div class="week">
      <div class="cost">
        <p class="label">{t("overview-cost")}</p>
        <p class="value readout" title={t("overview-cost-tip")}>
          {t("overview-cost-value", { usd: usd(overview.cost) })}
        </p>
        {#if change}
          <p class="change">
            <span class="dir readout">{t("overview-change", { dir: change.dir, pct: change.pct })}</span>
            {t("overview-change-basis")}
          </p>
        {:else}
          <p class="change">{t("overview-change-none")}</p>
        {/if}
        {#if overview.previous_cost != null}
          <p class="previous">{t("overview-previous", { usd: usd(overview.previous_cost) })}</p>
        {/if}
      </div>
      <div class="mix">
        <div>
          <h3>{t("overview-tokens")}</h3>
          <BarList items={tokens} label={t("overview-tokens")} />
        </div>
        {#if models.length > 0}
          <div>
            <h3>{t("overview-models")}</h3>
            <BarList items={models} label={t("overview-models")} />
          </div>
        {/if}
      </div>
    </div>
  {/if}
</Card>

<style>
  .since {
    font: var(--type-caption);
    color: var(--text-2);
  }
  .week {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 2fr);
    gap: var(--space-5);
  }
  .cost p {
    margin: 0;
  }
  .label,
  h3 {
    margin: 0 0 var(--space-1);
    font: var(--type-caption);
    font-weight: 600;
    color: var(--text-2);
  }
  .value {
    font-size: 28px;
    line-height: 34px;
    color: var(--text-1);
  }
  .change {
    margin-top: var(--space-1) !important;
    font: var(--type-body);
    color: var(--text-2);
  }
  .dir {
    color: var(--text-1);
    margin-inline-end: var(--space-1);
  }
  .previous,
  .note {
    font: var(--type-caption);
    color: var(--text-2);
  }
  .previous {
    margin-top: var(--space-2) !important;
  }
  .note {
    margin: 0;
  }
  .mix {
    display: grid;
    gap: var(--space-4);
    min-width: 0;
  }
  @container (max-width: 640px) {
    .week {
      grid-template-columns: 1fr;
    }
  }
</style>

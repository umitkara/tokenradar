<!--
  "How much value am I getting?" (Step 15b): this month's API-equivalent cost against the plan
  fee. Early in a month the ratio is low because the month has barely started, so the card says
  how much of it has gone and shows last month's finished ratio beside it. The fee is the user's
  (settings) or the embedded plan table's. For information only: not a bill.
-->
<script lang="ts">
import Card from "../../../lib/components/Card.svelte";
import { t } from "../../../lib/i18n/i18n.svelte";
import { commands, type PlanValue } from "../../../lib/ipc/bindings";
import { inTauri } from "../../../lib/ipc/runtime";
import { usd } from "./view";

let { plan }: { plan: PlanValue } = $props();

function edit() {
  if (inTauri()) void commands.openSettings("plan");
}
</script>

<Card title={t("overview-plan")}>
  <dl class="stats">
    <div>
      <dt>{t("overview-plan-month")}</dt>
      <dd class="readout">{t("overview-cost-value", { usd: usd(plan.month_cost) })}</dd>
    </div>
    {#if plan.fee != null}
      <div>
        <dt>{t("overview-plan-fee", { source: plan.fee_source ?? "table", plan: plan.plan ?? "" })}</dt>
        <dd class="readout">{t("overview-plan-usd", { usd: usd(plan.fee) })}</dd>
      </div>
    {/if}
    {#if plan.ratio != null}
      <div>
        <dt>{t("overview-plan-ratio-label")}</dt>
        <dd class="readout ratio">{t("overview-plan-ratio", { x: plan.ratio })}</dd>
      </div>
    {/if}
  </dl>
  {#if plan.fee == null}
    <p class="note">{t("overview-plan-no-fee")}</p>
  {:else}
    <p class="note">
      {t("overview-plan-elapsed", { pct: Math.round((plan.month_elapsed ?? 0) * 100) })}
      {#if plan.previous_ratio != null && plan.previous_month_cost != null}
        · {t("overview-plan-previous", { x: plan.previous_ratio, usd: usd(plan.previous_month_cost) })}
      {/if}
    </p>
  {/if}
  <p class="note">{t("overview-plan-note")}</p>
  <button type="button" class="link" onclick={edit}>{t("overview-plan-edit")}</button>
</Card>

<style>
  .stats {
    margin: 0 0 var(--space-3);
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3) var(--space-5);
  }
  dt {
    font: var(--type-caption);
    color: var(--text-2);
  }
  dd {
    margin: 0;
    font-size: 17px;
    line-height: 24px;
    color: var(--text-1);
  }
  .ratio {
    font-weight: 600;
  }
  .note {
    margin: 0 0 var(--space-1);
    font: var(--type-caption);
    color: var(--text-2);
  }
  .link {
    margin-top: var(--space-2);
    padding: 0;
    border: 0;
    background: none;
    font: var(--type-caption);
    color: var(--text-1);
    text-decoration: underline;
    text-underline-offset: 2px;
    cursor: pointer;
  }
  .link:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
    border-radius: var(--radius-control);
  }
</style>

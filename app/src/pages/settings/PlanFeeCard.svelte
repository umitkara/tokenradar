<!--
  The plan fee (Step 15b): what the overview's plan value compares this month's API-equivalent
  cost with. Empty means the embedded plan table's list price; a value is stored as the primary
  account's `plan_fee_usd` in config.toml.
-->
<script lang="ts">
import Button from "../../lib/components/Button.svelte";
import Card from "../../lib/components/Card.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import { commands, type PlanFeeView } from "../../lib/ipc/bindings";
import Field from "./Field.svelte";
import { store } from "./state.svelte";
import { parseFee } from "./view";

let { fee, onerror }: { fee: PlanFeeView; onerror: (key: string | null) => void } = $props();

let text = $state("");
let invalid = $state(false);
let busy = $state(false);
let saved = $state(false);

$effect(() => {
  text = fee.user_fee_usd != null ? String(fee.user_fee_usd) : "";
});

async function save(value: number | null) {
  busy = true;
  saved = false;
  const r = await commands.setPlanFee(value);
  busy = false;
  if (r.status === "ok") {
    store.sources = r.data;
    saved = true;
    onerror(null);
  } else {
    onerror("settings-save-failed");
  }
}

function submit(e: SubmitEvent) {
  e.preventDefault();
  const parsed = parseFee(text);
  invalid = parsed === undefined;
  if (parsed !== undefined) void save(parsed);
}
</script>

<Card title={t("plan-title")}>
  <p class="hint">{t("plan-hint")}</p>
  <p class="hint">
    {fee.table_plan != null && fee.table_fee_usd != null
      ? t("plan-table", { plan: fee.table_plan, usd: fee.table_fee_usd })
      : t("plan-table-none")}
  </p>
  <form onsubmit={submit}>
    <Field label={t("plan-fee")} id="plan-fee" hint={t("plan-fee-hint")}>
      <div class="row">
        <input
          id="plan-fee"
          inputmode="decimal"
          maxlength="9"
          placeholder={fee.table_fee_usd != null ? String(fee.table_fee_usd) : ""}
          aria-invalid={invalid}
          aria-describedby={invalid ? "plan-fee-error" : undefined}
          disabled={busy}
          bind:value={text}
          oninput={() => {
            invalid = false;
            saved = false;
          }}
        />
        <Button type="submit" variant="primary" disabled={busy}>{t("plan-save")}</Button>
        {#if fee.user_fee_usd != null}
          <Button disabled={busy} onclick={() => void save(null)}>{t("plan-reset")}</Button>
        {/if}
      </div>
    </Field>
  </form>
  {#if invalid}
    <p id="plan-fee-error" class="error" role="alert">{t("plan-invalid")}</p>
  {:else if saved}
    <p class="note" role="status">{t("plan-saved")}</p>
  {/if}
</Card>

<style>
  .hint {
    color: var(--text-2);
    margin-bottom: var(--space-2);
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  input {
    width: 96px;
    font-variant-numeric: tabular-nums;
  }
  .error,
  .note {
    margin-top: var(--space-1);
    font: var(--type-caption);
    color: var(--text-2);
  }
  .error {
    color: var(--text-1);
    border-left: 2px solid var(--signal-crit);
    padding-left: var(--space-2);
  }
</style>

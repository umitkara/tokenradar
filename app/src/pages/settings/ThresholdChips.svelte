<!-- One list of notification thresholds: removable chips and a field to add one. -->
<script lang="ts">
import { t } from "../../lib/i18n/i18n.svelte";
import { addThreshold, MAX_THRESHOLDS, parseThreshold, removeThreshold } from "./view";

interface Props {
  id: string;
  label: string;
  values: readonly number[];
  disabled?: boolean;
  onchange: (values: number[]) => void;
}

let { id, label, values, disabled = false, onchange }: Props = $props();

let text = $state("");
let invalid = $state<string | null>(null);
const full = $derived(values.length >= MAX_THRESHOLDS);

function add(e: SubmitEvent) {
  e.preventDefault();
  const value = parseThreshold(text);
  if (value === null) {
    invalid = t("notify-threshold-invalid");
    return;
  }
  const next = addThreshold(values, value);
  if (!next) {
    invalid = t("notify-threshold-full", { max: MAX_THRESHOLDS });
    return;
  }
  invalid = null;
  text = "";
  onchange(next);
}
</script>

<div class="chips" role="group" aria-labelledby="{id}-label">
  <span id="{id}-label" class="label">{label}</span>
  <ul>
    {#each values as value (value)}
      <li>
        <span>{t("value-pct", { pct: value, estimate: "no" })}</span>
        <button
          type="button"
          aria-label={t("notify-threshold-remove", { pct: value })}
          {disabled}
          onclick={() => onchange(removeThreshold(values, value))}>×</button
        >
      </li>
    {/each}
  </ul>
  <form onsubmit={add}>
    <input
      id="{id}-new"
      inputmode="numeric"
      maxlength="4"
      aria-label={t("notify-threshold-new")}
      aria-invalid={invalid !== null}
      aria-describedby={invalid ? `${id}-error` : undefined}
      disabled={disabled || full}
      bind:value={text}
    />
    <button type="submit" class="add" disabled={disabled || full}>{t("notify-threshold-add")}</button>
  </form>
  {#if invalid}<p id="{id}-error" class="error" role="alert">{invalid}</p>{/if}
</div>

<style>
  .chips {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) 0;
  }
  .label {
    min-width: 120px;
  }
  ul {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }
  li {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    padding: 0 2px 0 var(--space-2);
    min-height: 24px;
    border-radius: var(--radius-pill);
    border: 1px solid var(--stroke);
    background: var(--surface-2);
    font-family: var(--font-mono);
    font-size: 12px;
  }
  li button {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    color: var(--text-2);
    font-family: var(--font-ui);
  }
  li button:hover:not(:disabled) {
    background: var(--surface-1);
    color: var(--text-1);
  }
  form {
    display: inline-flex;
    gap: var(--space-1);
  }
  input {
    width: 56px;
    min-height: 24px;
    padding: 0 var(--space-2);
    border-radius: var(--radius-control);
    border: 1px solid var(--stroke);
    background: var(--surface-2);
    user-select: text;
  }
  input:disabled,
  button:disabled {
    opacity: 0.5;
  }
  .add {
    min-height: 24px;
    padding: 0 var(--space-2);
    border-radius: var(--radius-control);
    border: 1px solid var(--stroke);
    background: var(--surface-2);
  }
  .error {
    flex-basis: 100%;
    font: var(--type-caption);
    color: var(--text-1);
    border-left: 2px solid var(--signal-crit);
    padding-left: var(--space-2);
  }
</style>

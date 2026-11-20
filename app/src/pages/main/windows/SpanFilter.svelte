<!--
  The range picker of the windows and patterns pages (Step 17a): 4 weeks, 12 weeks, all, or two
  local days. It only reports changes; the page writes them into the URL fragment.
-->
<script lang="ts">
import Segmented from "../../../lib/components/Segmented.svelte";
import { t } from "../../../lib/i18n/i18n.svelte";
import { customDefault, SPAN_PRESETS, type SpanPreset, type SpanQuery } from "./filter";

interface Props {
  value: SpanQuery;
  onchange: (change: Partial<SpanQuery>) => void;
}

let { value, onchange }: Props = $props();

const options = $derived(SPAN_PRESETS.map((v) => ({ value: v, label: t(`span-${v}`) })));

function setRange(range: SpanPreset) {
  if (range === "custom") onchange({ range, ...customDefault(value, new Date()) });
  else onchange({ range, from: null, to: null });
}

function setDay(which: "from" | "to", day: string) {
  if (day) onchange({ [which]: day });
}
</script>

<Segmented value={value.range} {options} label={t("sessions-range")} onchange={setRange} />
{#if value.range === "custom"}
  <label class="day">
    <span>{t("sessions-from")}</span>
    <input type="date" value={value.from ?? ""} onchange={(e) => setDay("from", e.currentTarget.value)} />
  </label>
  <label class="day">
    <span>{t("sessions-to")}</span>
    <input type="date" value={value.to ?? ""} onchange={(e) => setDay("to", e.currentTarget.value)} />
  </label>
{/if}

<style>
  .day {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font: var(--type-caption);
    color: var(--text-2);
  }
  .day input {
    min-height: 32px;
    padding: 0 var(--space-2);
    border-radius: var(--radius-control);
    border: 1px solid var(--stroke);
    background: var(--surface-1);
    color: var(--text-1);
    font: var(--type-body);
  }
</style>

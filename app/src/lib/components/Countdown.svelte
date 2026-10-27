<!-- Time until a reset, from the shared 1 Hz clock (lib/time/clock.svelte.ts). `relative`:
     "Resets in 1h 48m"; `short`: "1h 48m"; `absolute`: "Resets Wed 10:59" for resets days away. -->
<script lang="ts">
import { weekdayClock } from "../i18n/format";
import { hour12, locale, t } from "../i18n/i18n.svelte";
import { clock } from "../time/clock.svelte";

interface Props {
  /** Reset time, milliseconds since the epoch; nothing renders without one. */
  at: number | null;
  mode?: "relative" | "short" | "absolute";
}

let { at, mode = "relative" }: Props = $props();

const text = $derived.by(() => {
  if (at == null) return "";
  if (mode === "absolute") {
    const d = new Date(at);
    return t("resets-at", {
      at: weekdayClock(locale(), d.getDay(), d.getHours(), d.getMinutes(), hour12()),
    });
  }
  const ms = at - clock.now;
  if (ms <= 0) return t("resets-now");
  return t(mode === "short" ? "resets-in-short" : "resets-in", { ms });
});
</script>

{#if at != null}
  <span class="countdown">{text}</span>
{/if}

<style>
  .countdown {
    font-variant-numeric: tabular-nums;
  }
</style>

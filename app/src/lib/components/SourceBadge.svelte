<!-- Where a number came from (honest numbers). The tooltip says how old it is and, when a
     second source lifted the value, that both were combined (ADR-060). -->
<script lang="ts">
import { t } from "../i18n/i18n.svelte";
import type { Source } from "../ipc/bindings";
import { clock } from "../time/clock.svelte";
import Tooltip from "./Tooltip.svelte";

interface Props {
  source: Source;
  /** When the value was observed, milliseconds since the epoch. */
  observedAt: number;
  /** Another source whose reading lifted the value (`LimitView.combined_with`). */
  combinedWith?: Source | null;
}

let { source, observedAt, combinedWith = null }: Props = $props();

const KEYS: Record<Source, string> = {
  statusline: "source-statusline",
  api: "source-api",
  estimate: "source-estimate",
  codex_log: "source-codex-log",
};

const updated = $derived.by(() => {
  const ms = clock.now - observedAt;
  return ms < 60_000 ? t("source-updated-now") : t("source-updated", { ms });
});

const tip = $derived(combinedWith ? `${updated}. ${t("source-combined")}` : updated);
</script>

<Tooltip text={tip}>
  <!-- Focusable so keyboard users can reach the tooltip. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <span class="badge {source}" tabindex="0">{t(KEYS[source])}</span>
</Tooltip>

<style>
  .badge {
    display: inline-block;
    padding: 1px var(--space-2);
    border-radius: var(--radius-pill);
    border: 1px solid var(--stroke);
    font: var(--type-micro);
    color: var(--text-2);
    white-space: nowrap;
  }
  .statusline {
    color: var(--text-1);
  }
  .estimate {
    border-style: dashed;
    border-color: var(--text-2);
  }
</style>

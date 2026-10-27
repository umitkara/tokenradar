<!-- Severity is never color alone (design-language.md §2): warn is a triangle, crit a disc
     with a bar. `ok` draws nothing. -->
<script lang="ts">
import { t } from "../i18n/i18n.svelte";
import type { Severity } from "../ipc/bindings";

let { severity }: { severity: Severity } = $props();
</script>

{#if severity !== "ok"}
  <svg
    class="icon {severity}"
    viewBox="0 0 12 12"
    width="12"
    height="12"
    role="img"
    aria-label={t(severity === "warn" ? "severity-warn" : "severity-crit")}
  >
    {#if severity === "warn"}
      <path d="M6 1 11.2 10.6H.8Z" fill="currentColor" />
      <path d="M6 4.4v3M6 8.8v.2" class="mark" />
    {:else}
      <circle cx="6" cy="6" r="5.2" fill="currentColor" />
      <path d="M3.4 6h5.2" class="mark" />
    {/if}
  </svg>
{/if}

<style>
  .icon {
    flex: none;
  }
  .warn {
    color: var(--signal-warn);
  }
  .crit {
    color: var(--signal-crit);
  }
  .mark {
    stroke: var(--surface-0-solid);
    stroke-width: 1.4;
    stroke-linecap: round;
  }
</style>

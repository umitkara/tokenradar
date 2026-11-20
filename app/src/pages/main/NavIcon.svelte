<!-- The main window's 16 px navigation icons, in the panel icons' line style. Decorative: the
     link text carries the meaning. -->
<script lang="ts">
import type { MainRoute } from "../../lib/ipc/bindings";

let { name }: { name: MainRoute | "settings" } = $props();
</script>

<svg
  width="16"
  height="16"
  viewBox="0 0 16 16"
  fill="none"
  stroke="currentColor"
  stroke-width="1.25"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if name === "overview"}
    <!-- The radar: two rings and a sweep. -->
    <circle cx="8" cy="8" r="6" />
    <circle cx="8" cy="8" r="3" />
    <path d="M8 8l4.2-4.2" />
  {:else if name === "windows"}
    <!-- Limit windows on a time line. -->
    <rect x="2" y="3" width="5" height="10" rx="1" />
    <rect x="9" y="3" width="5" height="10" rx="1" />
    <path d="M4.5 10V8M11.5 10V6" />
  {:else if name === "sessions"}
    <!-- A terminal prompt. -->
    <rect x="1.5" y="2.5" width="13" height="11" rx="1.5" />
    <path d="M4.5 6.5l2 1.5-2 1.5M8.5 10h3" />
  {:else if name === "projects"}
    <path d="M2 4.5h4l1.5 1.5H14v6.5H2Z" />
  {:else if name === "patterns"}
    <!-- A heat map grid. -->
    <rect x="2" y="2" width="5" height="5" rx="1" />
    <rect x="9" y="2" width="5" height="5" rx="1" />
    <rect x="2" y="9" width="5" height="5" rx="1" />
    <rect x="9" y="9" width="5" height="5" rx="1" />
  {:else}
    <circle cx="8" cy="8" r="2" />
    <path
      d="M8 1.5v1.6M8 12.9v1.6M1.5 8h1.6M12.9 8h1.6M3.4 3.4l1.1 1.1M11.5 11.5l1.1 1.1M3.4 12.6l1.1-1.1M11.5 4.5l1.1-1.1"
    />
    <circle cx="8" cy="8" r="4.6" />
  {/if}
</svg>

<style>
  svg {
    flex: none;
    display: block;
  }
</style>

<!-- The panel's 16 px line icons. Decorative: the button or text next to them carries the meaning. -->
<script lang="ts">
type Name = "gear" | "refresh" | "bell" | "bell-off" | "cloud-off" | "info" | "folder";

let { name }: { name: Name } = $props();
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
  {#if name === "gear"}
    <circle cx="8" cy="8" r="2" />
    <path
      d="M8 1.5v1.6M8 12.9v1.6M1.5 8h1.6M12.9 8h1.6M3.4 3.4l1.1 1.1M11.5 11.5l1.1 1.1M3.4 12.6l1.1-1.1M11.5 4.5l1.1-1.1"
    />
    <circle cx="8" cy="8" r="4.6" />
  {:else if name === "refresh"}
    <path d="M13 8a5 5 0 1 1-1.5-3.6" />
    <path d="M13 2.5v2.5h-2.5" />
  {:else if name === "bell"}
    <path d="M4 11V7a4 4 0 0 1 8 0v4l1 1.5H3Z" />
    <path d="M6.5 14h3" />
  {:else if name === "bell-off"}
    <path d="M5 4.2A4 4 0 0 1 12 7v3M4 7v4l-1 1.5h9" />
    <path d="M6.5 14h3M2 2l12 12" />
  {:else if name === "cloud-off"}
    <path d="M4.5 12.5h7a2.5 2.5 0 0 0 .6-4.9A4 4 0 0 0 5 6.2a3.2 3.2 0 0 0-.5 6.3Z" />
    <path d="M6.5 8.5l3 3M9.5 8.5l-3 3" />
  {:else if name === "info"}
    <circle cx="8" cy="8" r="6" />
    <path d="M8 7.2v3.6M8 5h0" />
  {:else if name === "folder"}
    <path d="M2 4.5h4l1.5 1.5H14v6.5H2Z" />
  {/if}
</svg>

<style>
  svg {
    flex: none;
    display: block;
  }
</style>

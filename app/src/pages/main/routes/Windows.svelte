<!--
  The windows page (Step 17a): the views at `#windows?filters`, one window's drawer at
  `#windows/<id>?filters` over them.
-->
<script lang="ts">
import WindowDrawer from "../windows/WindowDrawer.svelte";
import WindowsPage from "../windows/WindowsPage.svelte";

let { param, query }: { param: string | null; query: string } = $props();

const id = $derived.by(() => {
  const n = Number(param);
  return param != null && Number.isSafeInteger(n) && n > 0 ? n : null;
});
</script>

<WindowsPage {query} />
{#if id != null}
  {#key id}
    <WindowDrawer {id} {query} />
  {/key}
{/if}

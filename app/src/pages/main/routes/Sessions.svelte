<!--
  The sessions page (Step 16a): the list at `#sessions?filters`, one session at `#sessions/<id>`.
-->
<script lang="ts">
import SessionDetail from "../sessions/SessionDetail.svelte";
import SessionList from "../sessions/SessionList.svelte";

let { param, query }: { param: string | null; query: string } = $props();

const id = $derived.by(() => {
  const n = Number(param);
  return param != null && Number.isSafeInteger(n) && n > 0 ? n : null;
});
</script>

{#if id != null}
  {#key id}
    <SessionDetail {id} />
  {/key}
{:else}
  <SessionList {query} />
{/if}

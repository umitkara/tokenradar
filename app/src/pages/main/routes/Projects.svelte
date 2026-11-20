<!--
  The projects page (Step 16b): the list at `#projects?filters`, one project at
  `#projects/<id>?sessions-filters`.
-->
<script lang="ts">
import ProjectDetail from "../projects/ProjectDetail.svelte";
import ProjectList from "../projects/ProjectList.svelte";

let { param, query }: { param: string | null; query: string } = $props();

const id = $derived.by(() => {
  const n = Number(param);
  return param != null && Number.isSafeInteger(n) && n > 0 ? n : null;
});
</script>

{#if id != null}
  {#key id}
    <ProjectDetail {id} {query} />
  {/key}
{:else}
  <ProjectList {query} />
{/if}

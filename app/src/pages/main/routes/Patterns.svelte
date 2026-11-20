<!-- The patterns page (Step 17a) at `#patterns?filters`. -->
<script lang="ts">
import PatternsPage from "../patterns/PatternsPage.svelte";

let { query }: { param: string | null; query: string } = $props();
</script>

<PatternsPage {query} />

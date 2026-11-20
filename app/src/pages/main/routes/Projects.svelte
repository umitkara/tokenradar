<!-- The projects page: an empty state until it is built. -->
<script lang="ts">
import Soon from "./Soon.svelte";

/** `#route/param`: unused until the page has detail views. */
let _props: { param: string | null } = $props();
</script>

<Soon route="projects" />

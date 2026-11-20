<!-- A page that arrives in a later version: what it will show. -->
<script lang="ts">
import EmptyState from "../../../lib/components/EmptyState.svelte";
import { t } from "../../../lib/i18n/i18n.svelte";
import type { MainRoute } from "../../../lib/ipc/bindings";

let { route }: { route: MainRoute } = $props();
</script>

<EmptyState title={t(`main-soon-${route}`)} body={t(`main-soon-${route}-body`)} />

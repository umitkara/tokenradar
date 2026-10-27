<!-- Placeholder until Step 10 builds the real panel. -->
<script lang="ts">
import { t } from "../../lib/i18n/i18n.svelte";
</script>

<main>
  <h1>{t("app-name")}</h1>
  <p>{t("loading")}</p>
</main>

<style>
  main {
    padding: var(--space-4);
    color: var(--text-1);
  }
  h1 {
    font: var(--type-title);
  }
</style>

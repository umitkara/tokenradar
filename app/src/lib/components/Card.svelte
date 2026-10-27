<!-- A translucent surface over Mica. `title` and `corner` (e.g. a SourceBadge) form the header. -->
<script lang="ts">
import type { Snippet } from "svelte";

interface Props {
  title?: string;
  corner?: Snippet;
  children: Snippet;
}

let { title, corner, children }: Props = $props();
</script>

<section class="card">
  {#if title || corner}
    <header>
      {#if title}<h2>{title}</h2>{/if}
      {#if corner}<div class="corner">{@render corner()}</div>{/if}
    </header>
  {/if}
  {@render children()}
</section>

<style>
  .card {
    background: var(--surface-1);
    border: 1px solid var(--stroke);
    border-radius: var(--radius-card);
    padding: var(--space-3);
  }
  header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-2);
  }
  h2 {
    margin: 0;
    font: var(--type-title);
    color: var(--text-1);
  }
  .corner {
    margin-left: auto;
  }
</style>

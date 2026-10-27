<!-- What is missing and what to do next (design-language.md §9). The action is optional. -->
<script lang="ts">
import type { Snippet } from "svelte";

interface Props {
  title: string;
  body?: string;
  action?: Snippet;
}

let { title, body, action }: Props = $props();
</script>

<div class="empty">
  <p class="title">{title}</p>
  {#if body}<p class="body">{body}</p>{/if}
  {#if action}<div class="action">{@render action()}</div>{/if}
</div>

<style>
  .empty {
    display: grid;
    justify-items: start;
    gap: var(--space-1);
    padding: var(--space-3) 0;
  }
  p {
    margin: 0;
  }
  .title {
    font: var(--type-title);
    color: var(--text-1);
  }
  .body {
    font: var(--type-body);
    color: var(--text-2);
  }
  .action {
    margin-top: var(--space-2);
  }
</style>

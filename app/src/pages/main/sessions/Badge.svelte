<!--
  A small label on a session: a model, an effort level, fast mode, a deleted transcript. Text
  wears text tokens; `tone` only changes the outline (design-language.md §4, badges).
-->
<script lang="ts">
import type { Snippet } from "svelte";

interface Props {
  tone?: "neutral" | "accent" | "muted";
  children: Snippet;
}

let { tone = "neutral", children }: Props = $props();
</script>

<span class="badge {tone}">{@render children()}</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    max-width: 100%;
    padding: 0 6px;
    min-height: 18px;
    border-radius: var(--radius-control);
    border: 1px solid var(--stroke);
    font: var(--type-caption);
    color: var(--text-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .accent {
    border-color: color-mix(in oklch, var(--accent) 60%, transparent);
    color: var(--text-1);
  }
  .muted {
    border-style: dashed;
    color: var(--text-3);
  }
</style>

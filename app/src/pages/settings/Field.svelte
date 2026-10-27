<!--
  One setting: a label and an optional hint on the left, the control on the right. `id` is the
  control's id, so the label is its accessible name.
-->
<script lang="ts">
import type { Snippet } from "svelte";

interface Props {
  label: string;
  /** The control's id; omit for controls that carry their own label (a toggle). */
  id?: string;
  hint?: string;
  children: Snippet;
}

let { label, id, hint, children }: Props = $props();
</script>

<div class="field">
  <div class="text">
    {#if id}<label for={id}>{label}</label>{:else}<span>{label}</span>{/if}
    {#if hint}<p class="hint">{hint}</p>{/if}
  </div>
  <div class="control">{@render children()}</div>
</div>

<style>
  .field {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-2) 0;
  }
  .field + :global(.field) {
    border-top: 1px solid var(--stroke);
  }
  .text {
    flex: 1;
    min-width: 0;
  }
  .hint {
    margin-top: 2px;
    font: var(--type-caption);
    color: var(--text-2);
  }
  .control {
    flex: none;
  }
</style>

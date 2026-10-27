<!-- A square icon button. `label` is the accessible name and the tooltip. -->
<script lang="ts">
import type { Snippet } from "svelte";
import type { HTMLButtonAttributes } from "svelte/elements";
import Tooltip from "./Tooltip.svelte";

interface Props extends HTMLButtonAttributes {
  label: string;
  placement?: "above" | "below";
  children: Snippet;
}

let { label, placement, children, ...rest }: Props = $props();
</script>

<Tooltip text={label} {placement}>
  <button type="button" aria-label={label} {...rest}>{@render children()}</button>
</Tooltip>

<style>
  button {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: 1px solid transparent;
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--text-2);
    cursor: pointer;
    transition: background-color var(--dur-fast) var(--ease-out);
  }
  button:hover:not(:disabled) {
    background: var(--surface-2);
    color: var(--text-1);
  }
  button:focus-visible {
    color: var(--text-1);
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>

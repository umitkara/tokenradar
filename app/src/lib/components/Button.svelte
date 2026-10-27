<!-- A text button. `primary` fills with the accent; use it once per view at most. -->
<script lang="ts">
import type { Snippet } from "svelte";
import type { HTMLButtonAttributes } from "svelte/elements";

interface Props extends HTMLButtonAttributes {
  variant?: "primary" | "secondary";
  children: Snippet;
}

let { variant = "secondary", type = "button", children, ...rest }: Props = $props();
</script>

<button class={variant} {type} {...rest}>{@render children()}</button>

<style>
  button {
    font: var(--type-body);
    min-height: 28px;
    padding: 0 var(--space-3);
    border-radius: var(--radius-control);
    border: 1px solid var(--stroke);
    cursor: pointer;
    transition: background-color var(--dur-fast) var(--ease-out);
  }
  .secondary {
    background: var(--surface-2);
    color: var(--text-1);
  }
  .secondary:hover:not(:disabled) {
    background: var(--surface-1);
  }
  .primary {
    background: var(--accent);
    border-color: transparent;
    color: var(--on-accent);
    font-weight: 600;
  }
  .primary:hover:not(:disabled) {
    filter: brightness(1.06);
  }
  button:active:not(:disabled) {
    filter: brightness(0.94);
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  @media (forced-colors: active) {
    button {
      border-color: ButtonText;
    }
  }
</style>

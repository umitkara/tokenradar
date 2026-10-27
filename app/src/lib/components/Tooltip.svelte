<!-- A text tooltip for its children, on hover and keyboard focus; Esc hides it (and only it:
     the panel's own Esc handler skips a handled key). The text is also the trigger's
     description for screen readers. `below` suits triggers at the top of a window. -->
<script lang="ts">
import type { Snippet } from "svelte";

interface Props {
  text: string;
  placement?: "above" | "below";
  children: Snippet;
}

let { text, placement = "above", children }: Props = $props();
const id = $props.id();
let open = $state(false);
let timer: ReturnType<typeof setTimeout> | undefined;

function show(delay: number) {
  clearTimeout(timer);
  timer = setTimeout(() => {
    open = true;
  }, delay);
}
function hide() {
  clearTimeout(timer);
  open = false;
}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<span
  class="anchor"
  aria-describedby={id}
  onpointerenter={() => show(400)}
  onpointerleave={hide}
  onfocusin={() => show(0)}
  onfocusout={hide}
  onkeydown={(e) => {
    if (e.key === "Escape" && open) {
      hide();
      e.preventDefault();
    }
  }}
>
  {@render children()}
  <span {id} role="tooltip" class="tip {placement}" class:open>{text}</span>
</span>

<style>
  .anchor {
    position: relative;
    display: inline-flex;
  }
  .tip {
    position: absolute;
    bottom: calc(100% + var(--space-1));
    left: 50%;
    translate: -50% 0;
    z-index: 10;
    width: max-content;
    max-width: 240px;
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-control);
    border: 1px solid var(--stroke);
    background: var(--surface-0-solid);
    color: var(--text-1);
    font: var(--type-caption);
    pointer-events: none;
    opacity: 0;
    visibility: hidden;
    transition: opacity var(--dur-fast) var(--ease-out);
  }
  .tip.below {
    bottom: auto;
    top: calc(100% + var(--space-1));
  }
  /* Keep the tip inside narrow windows: anchored to the trigger's end instead of its middle. */
  :global(.tip-end) .tip {
    left: auto;
    right: 0;
    translate: none;
  }
  .tip.open {
    opacity: 1;
    visibility: visible;
  }
  @media (forced-colors: active) {
    .tip {
      border-color: CanvasText;
    }
  }
</style>

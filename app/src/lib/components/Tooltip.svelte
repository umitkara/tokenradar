<!-- A text tooltip for its children, on hover and keyboard focus; Esc hides it. The text is
     also the trigger's description for screen readers. -->
<script lang="ts">
import type { Snippet } from "svelte";

interface Props {
  text: string;
  children: Snippet;
}

let { text, children }: Props = $props();
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
    if (e.key === "Escape") hide();
  }}
>
  {@render children()}
  <span {id} role="tooltip" class="tip" class:open>{text}</span>
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

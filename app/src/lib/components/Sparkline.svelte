<!--
  A small trend line (Step 15b): daily values, oldest first, as an SVG polyline in muted ink
  with the last point marked in the accent. Decorative next to the number it belongs to, so it
  is hidden from screen readers unless given a label. No axes, no animation.
-->
<script lang="ts">
import { sparkPoints } from "../chart/spark";

interface Props {
  values: readonly number[];
  width?: number;
  height?: number;
  /** Screen reader text; without it the line is decorative. */
  label?: string;
}

let { values, width = 64, height = 20, label }: Props = $props();

const points = $derived(sparkPoints(values, width, height));
const last = $derived(points.split(" ").at(-1)?.split(",").map(Number) ?? null);
</script>

<svg
  class="spark"
  {width}
  {height}
  viewBox="0 0 {width} {height}"
  role={label ? "img" : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : "true"}
>
  {#if points}
    <polyline {points} />
    {#if last && last.length === 2}
      <circle cx={last[0]} cy={last[1]} r="2" />
    {/if}
  {/if}
</svg>

<style>
  .spark {
    display: block;
    overflow: visible;
  }
  polyline {
    fill: none;
    stroke: var(--text-3);
    stroke-width: 1.5;
    stroke-linejoin: round;
    stroke-linecap: round;
  }
  circle {
    fill: var(--accent);
  }
  @media (forced-colors: active) {
    polyline {
      stroke: CanvasText;
    }
    circle {
      fill: Highlight;
    }
  }
</style>

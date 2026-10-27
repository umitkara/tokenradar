<!--
  The indexing ring: one arc filling clockwise from 12 o'clock. `pct = null` draws the track
  only (not started). Value changes ease over --dur-slow; nothing moves otherwise (ADR-019).
-->
<script lang="ts">
interface Props {
  pct: number | null;
  /** Accessible name; the value is announced through aria-valuenow. */
  label: string;
  size?: number;
}

let { pct, label, size = 72 }: Props = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 72 72"
  role="progressbar"
  aria-label={label}
  aria-valuemin={0}
  aria-valuemax={100}
  aria-valuenow={pct ?? undefined}
>
  <circle class="track" cx="36" cy="36" r="30" />
  {#if pct !== null}
    <circle
      class="value"
      cx="36"
      cy="36"
      r="30"
      pathLength="100"
      stroke-dasharray="{pct} 100"
      transform="rotate(-90 36 36)"
    />
  {/if}
</svg>

<style>
  circle {
    fill: none;
    stroke-width: 6;
  }
  .track {
    stroke: var(--track);
  }
  .value {
    stroke: var(--accent);
    stroke-linecap: round;
    transition: stroke-dasharray var(--dur-slow) var(--ease-out);
  }
  @media (forced-colors: active) {
    .track {
      stroke: GrayText;
    }
    .value {
      stroke: Highlight;
    }
  }
</style>

<!--
  The charts' shared tooltip: a time (or other title) and one row per series with its color
  mark, label and value. Values are text tokens, never the series color (dataviz). A dashed
  mark and "≈" in the value mean an estimate. Positioned by the chart; it never takes the
  pointer, so it cannot get in the way of the cursor.
-->
<script lang="ts">
export interface TooltipRow {
  label: string;
  value: string;
  color: string;
  dashed?: boolean;
  /** Smaller, muted row (a note such as "Limit reached"). */
  note?: boolean;
}

interface Props {
  title: string;
  rows: TooltipRow[];
  /** Left edge in px inside the chart, already clamped. */
  x: number;
  y: number;
}

let { title, rows, x, y }: Props = $props();
</script>

<div class="tip" style:left="{x}px" style:top="{y}px" role="presentation">
  <p class="title">{title}</p>
  {#each rows as row, i (i)}
    <p class="row" class:note={row.note}>
      {#if !row.note}
        <span class="mark" class:dashed={row.dashed} style:--mark={row.color} aria-hidden="true"></span>
      {/if}
      <span class="label">{row.label}</span>
      <span class="value readout">{row.value}</span>
    </p>
  {/each}
</div>

<style>
  .tip {
    position: absolute;
    z-index: 5;
    pointer-events: none;
    min-width: 140px;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-control);
    border: 1px solid var(--stroke);
    background: var(--surface-2);
    box-shadow: 0 4px 12px oklch(0 0 0 / 0.18);
    font: var(--type-caption);
    color: var(--text-1);
    display: grid;
    gap: 2px;
  }
  p {
    margin: 0;
  }
  .title {
    color: var(--text-2);
    margin-bottom: 2px;
  }
  .row {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: var(--space-2);
  }
  .row.note {
    grid-template-columns: 1fr;
    color: var(--text-2);
  }
  .mark {
    width: 12px;
    height: 0;
    border-top: 2px solid var(--mark);
  }
  .mark.dashed {
    border-top-style: dashed;
  }
  .value {
    font-variant-numeric: tabular-nums;
    text-align: end;
  }
  @media (forced-colors: active) {
    .tip {
      border-color: CanvasText;
    }
  }
</style>

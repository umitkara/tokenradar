<!--
  A magnitude list (Step 15b): label, a thin bar sized against the largest row, and the value.
  One hue for every row (the job is magnitude, not identity); 4 px rounded data end on a 2 px
  track-less baseline, values in text ink (dataviz).
-->
<script lang="ts">
export interface BarItem {
  key: string;
  label: string;
  /** 0–100 of the largest row. */
  width: number;
  value: string;
  /** Hover text: the exact value and share. */
  title?: string;
}

let { items, label }: { items: BarItem[]; label: string } = $props();
</script>

<ul class="bars" aria-label={label}>
  {#each items as item (item.key)}
    <li title={item.title}>
      <span class="label">{item.label}</span>
      <span class="track" aria-hidden="true">
        {#if item.width > 0}
          <span class="bar" style:width="max(4px, {item.width}%)"></span>
        {/if}
      </span>
      <span class="value readout">{item.value}</span>
    </li>
  {/each}
</ul>

<style>
  .bars {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    grid-template-columns: minmax(0, max-content) minmax(40px, 1fr) max-content;
    column-gap: var(--space-3);
    row-gap: var(--space-2);
    align-items: center;
  }
  li {
    display: contents;
  }
  .label {
    font: var(--type-body);
    color: var(--text-1);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 220px;
  }
  .track {
    display: block;
    min-width: 40px;
  }
  .bar {
    display: block;
    height: 6px;
    border-radius: 0 3px 3px 0;
    background: var(--signal-ok);
  }
  .value {
    font-variant-numeric: tabular-nums;
    color: var(--text-2);
    text-align: end;
  }
  @media (forced-colors: active) {
    .bar {
      background: CanvasText;
    }
  }
</style>

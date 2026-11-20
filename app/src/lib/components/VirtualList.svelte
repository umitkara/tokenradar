<!--
  A long list that renders only the rows in view (Step 16a): fixed row height, so the slice is
  arithmetic (`lib/virtual.ts`) and 10,000 rows cost the same as 30.

  - An ARIA grid: the grid itself takes the focus and points at the active row with
    `aria-activedescendant`; rows carry `aria-rowindex` (the header is row 1) and the grid
    `aria-rowcount` (-1 while more pages may follow).
  - Keys: ↑ ↓, Page Up / Page Down, Home / End move the active row; Enter opens it. A click
    opens the row too. Other keys go to `onkey` (a tree's ← and →).
  - Paging: when the rendered slice comes within `overscan` rows of the end and `hasMore`,
    `onmore` is called (once per page; the caller sets `loading` while it fetches).
-->
<script lang="ts" generics="T">
import type { Snippet } from "svelte";
import { isNavKey, moveSelection, scrollToShow, visibleRange } from "../virtual";

interface Props {
  items: readonly T[];
  key: (item: T) => string | number;
  /** The grid's accessible name. */
  label: string;
  rowHeight?: number;
  /** CSS `grid-template-columns` shared by the header and every row. */
  columns: string;
  header: Snippet;
  row: Snippet<[T, number]>;
  hasMore?: boolean;
  loading?: boolean;
  onmore?: () => void;
  onactivate?: (item: T, index: number) => void;
  /** Other keys on the active row (a tree's ← and →); `true` when handled. */
  onkey?: (key: string, item: T, index: number) => boolean;
}

let {
  items,
  key,
  label,
  rowHeight = 44,
  columns,
  header,
  row,
  hasMore = false,
  loading = false,
  onmore,
  onactivate,
  onkey,
}: Props = $props();

const OVERSCAN = 6;
const id = $props.id();
let scroller: HTMLDivElement;
let scrollTop = $state(0);
let viewport = $state(0);
let active = $state<number | null>(null);

const range = $derived(visibleRange(scrollTop, viewport, rowHeight, items.length, OVERSCAN));
const slice = $derived(items.slice(range.start, range.end));
const page = $derived(Math.max(1, Math.floor(viewport / rowHeight)));

// A shorter list (new filters) must not leave the active row or the scroll past its end.
$effect(() => {
  if (active != null && active >= items.length) active = items.length > 0 ? items.length - 1 : null;
});

$effect(() => {
  if (hasMore && !loading && viewport > 0 && range.end >= items.length - OVERSCAN) onmore?.();
});

function show(index: number) {
  const next = scrollToShow(index, scroller.scrollTop, scroller.clientHeight, rowHeight);
  if (next != null) scroller.scrollTop = next;
}

function onkeydown(e: KeyboardEvent) {
  if (isNavKey(e.key)) {
    const next = moveSelection(e.key, active, items.length, page);
    if (next == null) return;
    e.preventDefault();
    active = next;
    show(next);
  } else if (active != null) {
    const item = items[active];
    if (item === undefined) return;
    if (e.key === "Enter") {
      e.preventDefault();
      onactivate?.(item, active);
    } else if (!e.ctrlKey && !e.altKey && !e.metaKey && onkey?.(e.key, item, active)) {
      e.preventDefault();
    }
  }
}

function onclick(index: number) {
  active = index;
  const item = items[index];
  if (item !== undefined) onactivate?.(item, index);
}

/** Makes `index` the active row and scrolls it into view (a tree's ← to the parent). */
export function select(index: number) {
  if (index < 0 || index >= items.length) return;
  active = index;
  show(index);
}

/** Back to the top, for a new set of filters. */
export function reset() {
  active = null;
  if (scroller) scroller.scrollTop = 0;
}
</script>

<div
  class="grid"
  role="grid"
  tabindex="0"
  aria-label={label}
  aria-rowcount={hasMore ? -1 : items.length + 1}
  aria-activedescendant={active != null ? `${id}-r${active}` : undefined}
  style:--columns={columns}
  style:--row-h="{rowHeight}px"
  {onkeydown}
>
  <div class="head" role="row" aria-rowindex={1}>
    {@render header()}
  </div>
  <div
    class="scroller"
    bind:this={scroller}
    bind:clientHeight={viewport}
    onscroll={() => (scrollTop = scroller.scrollTop)}
  >
    <div class="spacer" style:height="{items.length * rowHeight}px">
      <div class="slice" style:transform="translateY({range.start * rowHeight}px)" role="rowgroup">
        {#each slice as item, i (key(item))}
          {@const index = range.start + i}
          <!-- Keyboard use goes through the grid (aria-activedescendant), not each row. -->
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <div
            class="row"
            class:active={active === index}
            role="row"
            tabindex="-1"
            id="{id}-r{index}"
            aria-rowindex={index + 2}
            aria-selected={active === index}
            onclick={() => onclick(index)}
          >
            {@render row(item, index)}
          </div>
        {/each}
      </div>
    </div>
  </div>
</div>

<style>
  .grid {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1 1 0;
    border-radius: var(--radius-card);
    border: 1px solid var(--stroke);
    background: var(--surface-1);
    overflow: hidden;
  }
  .grid:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
  .head,
  .row {
    display: grid;
    grid-template-columns: var(--columns);
    align-items: center;
    column-gap: var(--space-3);
    padding: 0 var(--space-3);
  }
  .head {
    min-height: 36px;
    border-bottom: 1px solid var(--stroke);
    font: var(--type-caption);
    color: var(--text-2);
  }
  .scroller {
    flex: 1 1 0;
    min-height: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  .spacer {
    position: relative;
  }
  .slice {
    position: absolute;
    inset: 0 0 auto 0;
    will-change: transform;
  }
  .row {
    height: var(--row-h);
    border-bottom: 1px solid color-mix(in oklch, var(--stroke) 60%, transparent);
    cursor: default;
    transition: background-color var(--dur-fast) var(--ease-out);
  }
  .row:hover {
    background: var(--surface-2);
  }
  .row.active {
    background: var(--surface-2);
    box-shadow: inset 2px 0 0 var(--accent);
  }
  @media (forced-colors: active) {
    .row.active {
      outline: 1px solid Highlight;
      outline-offset: -1px;
    }
  }
</style>

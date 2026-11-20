// The pure half of the virtual list (Step 16a): which rows to render for a scroll position, and
// where the keyboard moves the selection. Rows have one fixed height, so both are arithmetic; the
// component (`components/VirtualList.svelte`) only measures the viewport and renders the slice.

export interface Range {
  /** First rendered row, inclusive. */
  start: number;
  /** Last rendered row, exclusive. */
  end: number;
}

/**
 * Rows to render: the ones in `[scrollTop, scrollTop + viewport)` plus `overscan` rows on each
 * side, clamped to `[0, count)`.
 */
export function visibleRange(
  scrollTop: number,
  viewport: number,
  rowHeight: number,
  count: number,
  overscan = 6,
): Range {
  if (count <= 0 || rowHeight <= 0) return { start: 0, end: 0 };
  const first = Math.floor(Math.max(0, scrollTop) / rowHeight);
  const shown = Math.ceil(Math.max(0, viewport) / rowHeight) + 1;
  const start = Math.max(0, Math.min(count - 1, first - overscan));
  const end = Math.min(count, first + shown + overscan);
  return { start, end: Math.max(start, end) };
}

/** The keys the list handles; anything else is left to the page. */
export type NavKey = "ArrowUp" | "ArrowDown" | "PageUp" | "PageDown" | "Home" | "End";

export function isNavKey(key: string): key is NavKey {
  return ["ArrowUp", "ArrowDown", "PageUp", "PageDown", "Home", "End"].includes(key);
}

/**
 * The row selected after `key`. With nothing selected, any key but `End` selects the first row.
 * `page` is the number of rows a viewport shows. `null` for an empty list.
 */
export function moveSelection(
  key: NavKey,
  current: number | null,
  count: number,
  page: number,
): number | null {
  if (count <= 0) return null;
  const last = count - 1;
  if (current == null) return key === "End" ? last : 0;
  const step = Math.max(1, page - 1);
  const next = {
    ArrowUp: current - 1,
    ArrowDown: current + 1,
    PageUp: current - step,
    PageDown: current + step,
    Home: 0,
    End: last,
  }[key];
  return Math.min(last, Math.max(0, next));
}

/**
 * The scroll position that brings row `index` fully into view, or `null` when it already is.
 * Moves as little as possible: a row above the viewport aligns to the top, one below to the
 * bottom.
 */
export function scrollToShow(
  index: number,
  scrollTop: number,
  viewport: number,
  rowHeight: number,
): number | null {
  const top = index * rowHeight;
  const bottom = top + rowHeight;
  if (top < scrollTop) return top;
  if (bottom > scrollTop + viewport) return Math.max(0, bottom - viewport);
  return null;
}

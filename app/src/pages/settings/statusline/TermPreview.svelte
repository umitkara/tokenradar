<!--
  The statusline as a terminal draws it: the core's painted spans on the `--term-*` palette,
  never wrapped. `columns` sets the terminal's width (a dashed edge marks it); a wider line
  scrolls sideways; `reserved` shades the columns at its right that the line keeps free (Claude
  Code's and the user's margin, ADR-051). With `onpick`, a span selects its item, and hovering
  highlights the item's spans here and its row in the list (`hovered` comes back from the list
  the same way). Blank items (spaces, flexible spaces) get a tint so they can be seen.
-->
<script lang="ts">
import type { Span } from "../../../lib/ipc/bindings";
import { spanClasses } from "../view";

interface Props {
  lines: Span[][];
  label: string;
  columns?: number | null;
  reserved?: number;
  reservedLabel?: string;
  selected?: number | null;
  hovered?: number | null;
  onpick?: (item: number) => void;
  onhover?: (item: number | null) => void;
}

let {
  lines,
  label,
  columns = null,
  reserved = 0,
  reservedLabel,
  selected = null,
  hovered = null,
  onpick,
  onhover,
}: Props = $props();

function itemOf(e: Event): number | null {
  const el = (e.target as HTMLElement | null)?.closest<HTMLElement>("[data-item]");
  return el ? Number(el.dataset.item) : null;
}

const empty = (span: Span) => span.text.trim() === "" && span.bg === null;

/**
 * Items that draw only blanks: a space, a flexible space (not a solid bar's cells). The gaps
 * inside an item with text (the parts of `git_status`) are not tinted; its text shows it.
 */
const drawnItems = $derived(
  new Set(
    lines
      .flat()
      .filter((s) => !empty(s))
      .map((s) => s.item),
  ),
);

const blank = (span: Span) => span.item !== null && empty(span) && !drawnItems.has(span.item);

function click(e: MouseEvent) {
  const item = itemOf(e);
  if (item !== null) onpick?.(item);
}

function over(e: MouseEvent) {
  if (onhover) onhover(itemOf(e));
}
</script>

<!-- Mouse shortcuts only: the item list offers the same selection by keyboard. -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<!-- svelte-ignore a11y_mouse_events_have_key_events -->
<div
  class="term"
  class:interactive={!!onpick}
  aria-label={label}
  role="group"
  onclick={click}
  onmouseover={over}
  onmouseleave={() => onhover?.(null)}
>
  <div class="screen" style:min-width={columns ? `${columns}ch` : null} class:edge={!!columns}>
    {#each lines as line, i (i)}
      <div class="line">
        {#each line as span, j (j)}<span
            class={spanClasses(span)}
            class:hl={span.item !== null && span.item === hovered}
            class:sel={span.item !== null && span.item === selected}
            class:blank={blank(span)}
            data-item={span.item ?? undefined}>{span.text}</span
          >{/each}
      </div>
    {/each}{#if columns && reserved > 0}<div
        class="reserved"
        style:left="{columns - reserved}ch"
        style:width="{reserved}ch"
        title={reservedLabel}
        aria-hidden="true"
      ></div>{/if}
  </div>
</div>

<style>
  .term {
    overflow-x: auto;
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-control);
    border: 1px solid var(--stroke);
    background: var(--term-bg);
    color: var(--term-fg);
    font: 400 13px / 20px var(--font-term);
    user-select: text;
  }
  /* Its own stacking context, so the shaded columns can sit under the text. */
  .screen {
    position: relative;
    isolation: isolate;
    width: max-content;
    white-space: pre;
  }
  /* The columns kept free at the right: faint diagonal hatching up to the edge. */
  .reserved {
    position: absolute;
    z-index: -1;
    top: calc(-1 * var(--space-2));
    bottom: calc(-1 * var(--space-2));
    background: repeating-linear-gradient(
      -45deg,
      #ffffff14 0 2px,
      transparent 2px 6px
    );
  }
  /* The terminal's right edge at `columns`. */
  .edge {
    box-sizing: content-box;
    background-image: linear-gradient(to bottom, #ffffff40 50%, transparent 50%);
    background-size: 1px 4px;
    background-repeat: repeat-y;
    background-position: right top;
  }
  .line {
    position: relative;
    min-height: 20px;
  }
  .line span {
    color: var(--c, var(--term-fg));
  }
  .interactive [data-item] {
    cursor: pointer;
  }
  .hl {
    outline: 1px solid #ffffff99;
    outline-offset: -1px;
  }
  .sel {
    text-decoration: underline;
    text-decoration-color: var(--term-fg);
    text-underline-offset: 4px;
  }
  /* Blanks have no text to underline or outline: a tint shows where they are. */
  .blank.sel {
    background-color: #ffffff17;
  }
  .blank.hl {
    background-color: #ffffff26;
  }

  /* The paint's colors (render.rs `ansi`: ok cyan, warn yellow, crit red). */
  .p-ok {
    --c: var(--term-cyan);
  }
  .p-warn {
    --c: var(--term-yellow);
  }
  .p-crit {
    --c: var(--term-red);
  }
  .fg-black {
    --c: var(--term-black);
  }
  .fg-red {
    --c: var(--term-red);
  }
  .fg-green {
    --c: var(--term-green);
  }
  .fg-yellow {
    --c: var(--term-yellow);
  }
  .fg-blue {
    --c: var(--term-blue);
  }
  .fg-magenta {
    --c: var(--term-magenta);
  }
  .fg-cyan {
    --c: var(--term-cyan);
  }
  .fg-white {
    --c: var(--term-white);
  }
  .fg-gray {
    --c: var(--term-gray);
  }
  .fg-bright-red {
    --c: var(--term-bright-red);
  }
  .fg-bright-green {
    --c: var(--term-bright-green);
  }
  .fg-bright-yellow {
    --c: var(--term-bright-yellow);
  }
  .fg-bright-blue {
    --c: var(--term-bright-blue);
  }
  .fg-bright-magenta {
    --c: var(--term-bright-magenta);
  }
  .fg-bright-cyan {
    --c: var(--term-bright-cyan);
  }
  .fg-bright-white {
    --c: var(--term-bright-white);
  }
  /* SGR 2 fades the text only, never a block's background. */
  .line .p-dim {
    color: color-mix(in srgb, var(--c, var(--term-fg)) 55%, transparent);
  }
  .bg-black {
    background-color: var(--term-black);
  }
  .bg-red {
    background-color: var(--term-red);
  }
  .bg-green {
    background-color: var(--term-green);
  }
  .bg-yellow {
    background-color: var(--term-yellow);
  }
  .bg-blue {
    background-color: var(--term-blue);
  }
  .bg-magenta {
    background-color: var(--term-magenta);
  }
  .bg-cyan {
    background-color: var(--term-cyan);
  }
  .bg-white {
    background-color: var(--term-white);
  }
  .bg-gray {
    background-color: var(--term-gray);
  }
  .bg-bright-red {
    background-color: var(--term-bright-red);
  }
  .bg-bright-green {
    background-color: var(--term-bright-green);
  }
  .bg-bright-yellow {
    background-color: var(--term-bright-yellow);
  }
  .bg-bright-blue {
    background-color: var(--term-bright-blue);
  }
  .bg-bright-magenta {
    background-color: var(--term-bright-magenta);
  }
  .bg-bright-cyan {
    background-color: var(--term-bright-cyan);
  }
  .bg-bright-white {
    background-color: var(--term-bright-white);
  }
  @media (forced-colors: active) {
    .term {
      forced-color-adjust: none;
    }
  }
</style>

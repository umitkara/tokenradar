<!--
  The line's items, one card per output line (Step 12l). The list stays flat; the cards are a
  view of it, split at the line breaks the core draws (`lines`). A row selects its item; the
  grip drags it within a card or into another one (a static line marks where it lands,
  ADR-019) or moves it with ↑/↓; Alt+↑/↓ on a row moves it too, across a card's edge into the
  next line, and ↑/↓ alone walks the rows. Every move is announced with its line for screen
  readers.
-->
<script lang="ts">
import { tick } from "svelte";
import IconButton from "../../../lib/components/IconButton.svelte";
import { t } from "../../../lib/i18n/i18n.svelte";
import type {
  Item,
  SegmentKind,
  StatuslineKind,
  StatuslinePreview,
} from "../../../lib/ipc/bindings";
import {
  canDrop,
  canRemoveLine,
  dropIndex,
  flexHintKey,
  itemText,
  kindKey,
  lines,
  MAX_ITEMS,
  placeOf,
} from "../view";

interface Props {
  items: Item[];
  catalog: StatuslineKind[];
  preview: StatuslinePreview;
  selected: number | null;
  hovered: number | null;
  onselect: (index: number) => void;
  onhover: (index: number | null) => void;
  /** Moves the item at `from` to index `to` of the flat list. */
  onmove: (from: number, to: number) => void;
  /** Drops the item at `from` into gap `slot` of card `line`; resolves to its new index. */
  ondrop: (from: number, line: number, slot: number) => number | null;
  ontoggle: (index: number) => void;
  onremove: (index: number) => void;
  /** Opens the palette for card `line`. */
  onadd: (line: number) => void;
  onaddline: () => void;
  onremoveline: (line: number) => void;
}

let {
  items,
  catalog,
  preview,
  selected,
  hovered,
  onselect,
  onhover,
  onmove,
  ondrop,
  ontoggle,
  onremove,
  onadd,
  onaddline,
  onremoveline,
}: Props = $props();

let root: HTMLDivElement;
let drag = $state<{ from: number; line: number; slot: number } | null>(null);
let announce = $state("");

const cards = $derived(lines(items));
/** The row order across cards: ↑/↓ walks it. */
const order = $derived(cards.flatMap((c) => c.items));
const full = $derived(items.length >= MAX_ITEMS);

const name = (kind: SegmentKind) => t(kindKey(kind));
/** Layout items that draw no text of their own: quieter, thinner rows (Step 12l). */
const isThin = (kind: SegmentKind) =>
  kind === "separator" || kind === "space" || kind === "flex" || kind === "line_break";
const sampleOf = (kind: SegmentKind) => catalog.find((k) => k.kind === kind)?.sample ?? "";

/** The gap the drag would drop into, or `null` when it would not move anything. */
const dropAt = $derived.by(() => {
  if (!drag) return null;
  const place = placeOf(items, drag.from);
  if (!place) return null;
  const still = place.line === drag.line && dropIndex(place.position, drag.slot) === place.position;
  return still ? null : { line: drag.line, slot: drag.slot };
});

function rowText(item: Item, index: number): string {
  if (!item.enabled) return t("statusline-item-hidden");
  // Layout items show what they are; the core may leave a separator out of the preview.
  if (item.kind === "space") return t("statusline-columns", { n: item.width });
  if (item.kind === "separator") return item.text || "│";
  if (item.kind === "flex") return t(flexHintKey(items, index));
  if (item.kind === "line_break") return "";
  if (preview.absent.includes(item.kind)) return sampleOf(item.kind);
  return itemText(preview, index) ?? sampleOf(item.kind);
}

async function focusRow(index: number, part: "pick" | "grip" = "pick") {
  await tick();
  root.querySelector<HTMLElement>(`[data-row="${index}"] .${part}`)?.focus();
}

async function focusCard(line: number) {
  await tick();
  const card = root.querySelector<HTMLElement>(`[data-line="${line}"]`);
  (
    card?.querySelector<HTMLElement>(".pick") ?? card?.querySelector<HTMLElement>(".card-add")
  )?.focus();
}

function moved(index: number, list: Item[], kind: SegmentKind) {
  const place = placeOf(list, index);
  if (!place) return;
  announce = t("statusline-moved-line", {
    name: name(kind),
    line: place.line + 1,
    position: place.position + 1,
    count: place.count,
  });
}

/** Alt+↑/↓ and the grip's arrows: a swap with the neighbor, a line break included. */
async function step(from: number, by: -1 | 1, part: "pick" | "grip") {
  const item = items[from];
  const to = from + by;
  if (!item || to < 0 || to >= items.length) return;
  onmove(from, to);
  await tick();
  moved(to, items, item.kind);
  await focusRow(to, part);
}

function rowKey(e: KeyboardEvent, index: number) {
  const by = e.key === "ArrowUp" ? -1 : e.key === "ArrowDown" ? 1 : 0;
  if (by !== 0) {
    e.preventDefault();
    if (e.altKey) {
      void step(index, by, "pick");
      return;
    }
    const next = order[order.indexOf(index) + by];
    if (next !== undefined) {
      onselect(next);
      void focusRow(next);
    }
  } else if (e.key === "Delete" && canDrop(items, index)) {
    e.preventDefault();
    const at = order.indexOf(index);
    onremove(index);
    // The row after it takes its place; the last row's goes to the one before.
    const next = order[at + 1] !== undefined ? index : order[at - 1];
    if (next !== undefined) void focusRow(next);
  }
}

function gripKey(e: KeyboardEvent, index: number) {
  const by = e.key === "ArrowUp" ? -1 : e.key === "ArrowDown" ? 1 : 0;
  if (by === 0) return;
  e.preventDefault();
  void step(index, by, "grip");
}

/** The card and gap under `y`: a gap between cards belongs to the card below it. */
function slotAt(y: number): { line: number; slot: number } {
  const els = [...root.querySelectorAll<HTMLElement>("[data-line]")];
  for (const [n, card] of els.entries()) {
    if (y >= card.getBoundingClientRect().bottom && n < els.length - 1) continue;
    const rows = [...card.querySelectorAll<HTMLElement>("[data-row]")];
    const slot = rows.findIndex((row) => {
      const r = row.getBoundingClientRect();
      return y < r.top + r.height / 2;
    });
    return { line: Number(card.dataset.line), slot: slot < 0 ? rows.length : slot };
  }
  return { line: 0, slot: 0 };
}

function down(e: PointerEvent, index: number) {
  if (e.button !== 0) return;
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  const place = placeOf(items, index);
  if (!place) return;
  drag = { from: index, line: place.line, slot: place.position };
  onselect(index);
}

function moveDrag(e: PointerEvent) {
  if (drag) drag = { ...drag, ...slotAt(e.clientY) };
}

async function up() {
  if (!drag || !dropAt) {
    drag = null;
    return;
  }
  const { from, line, slot } = drag;
  const kind = items[from]?.kind;
  drag = null;
  const index = ondrop(from, line, slot);
  if (index === null || !kind) return;
  await tick();
  moved(index, items, kind);
  await focusRow(index, "grip");
}

function removeLine(line: number) {
  onremoveline(line);
  announce = t("statusline-line-removed", { n: line + 1 });
  void focusCard(Math.max(0, line - 1));
}

function addLine() {
  onaddline();
  announce = t("statusline-line-added", { n: cards.length + 1 });
  void focusCard(cards.length);
}

function cancelDrag(e: KeyboardEvent) {
  if (drag && e.key === "Escape") {
    e.preventDefault();
    e.stopPropagation();
    drag = null;
  }
}
</script>

<svelte:window onkeydown={cancelDrag} />

<div class="cards" bind:this={root} class:dragging={!!drag}>
  {#each cards as card, line (line)}
    {@const title = t("statusline-line", { n: line + 1 })}
    <section class="card" data-line={line} aria-label={title}>
      <header>
        <h3>{title}</h3>
        <IconButton
          label={t("statusline-line-remove", { n: line + 1 })}
          disabled={!canRemoveLine(items, line)}
          onclick={() => removeLine(line)}
        >
          <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
            <path d="M3.5 4.5h9M6.5 4.5V3h3v1.5M5 4.5l.5 8.5h5l.5-8.5" fill="none" stroke="currentColor" stroke-width="1.2" />
          </svg>
        </IconButton>
      </header>
      <ul aria-label={t("statusline-line-items", { n: line + 1 })}>
        {#each card.items as i, pos (i)}
          {@const item = items[i] as Item}
          {@const absent = preview.absent.includes(item.kind)}
          {@const droppable = canDrop(items, i)}
          {#if dropAt?.line === line && dropAt.slot === pos}<li class="drop" aria-hidden="true"></li>{/if}
          <li
            data-row={i}
            class:selected={selected === i}
            class:hovered={hovered === i}
            class:off={!item.enabled || absent}
            class:layout={isThin(item.kind)}
            class:lifted={drag?.from === i}
            onmouseenter={() => onhover(i)}
            onmouseleave={() => onhover(null)}
          >
            <button
              type="button"
              class="grip"
              aria-label={t("statusline-reorder", { name: name(item.kind) })}
              title={t("statusline-reorder", { name: name(item.kind) })}
              onpointerdown={(e) => down(e, i)}
              onpointermove={moveDrag}
              onpointerup={up}
              onpointercancel={() => (drag = null)}
              onkeydown={(e) => gripKey(e, i)}
              tabindex={selected === i ? 0 : -1}
            >
              <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
                <g fill="currentColor">
                  <circle cx="6" cy="4" r="1.1" /><circle cx="10" cy="4" r="1.1" />
                  <circle cx="6" cy="8" r="1.1" /><circle cx="10" cy="8" r="1.1" />
                  <circle cx="6" cy="12" r="1.1" /><circle cx="10" cy="12" r="1.1" />
                </g>
              </svg>
            </button>
            <button
              type="button"
              class="pick"
              aria-current={selected === i ? "true" : undefined}
              tabindex={selected === i || (selected === null && i === order[0]) ? 0 : -1}
              onclick={() => onselect(i)}
              onkeydown={(e) => rowKey(e, i)}
            >
              {#if item.kind === "flex"}
                <span class="mark" aria-hidden="true">⟷</span>
              {/if}
              <span class="name">{name(item.kind)}</span>
              <span class="text" class:hint={item.kind === "flex"}>{rowText(item, i)}</span>
              {#if absent}<span class="tag">{t("statusline-segment-absent")}</span>{/if}
            </button>
            <IconButton
              label={item.enabled ? t("statusline-hide") : t("statusline-show")}
              disabled={item.enabled && !droppable}
              onclick={() => ontoggle(i)}
              tabindex={selected === i ? 0 : -1}
            >
              {#if item.enabled}
                <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
                  <path d="M1.5 8s2.5-4.5 6.5-4.5S14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z" fill="none" stroke="currentColor" stroke-width="1.2" />
                  <circle cx="8" cy="8" r="2" fill="none" stroke="currentColor" stroke-width="1.2" />
                </svg>
              {:else}
                <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
                  <path d="M1.5 8s2.5-4.5 6.5-4.5S14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z" fill="none" stroke="currentColor" stroke-width="1.2" />
                  <path d="M2.5 13.5l11-11" stroke="currentColor" stroke-width="1.2" />
                </svg>
              {/if}
            </IconButton>
            <IconButton
              label={t("statusline-remove")}
              disabled={!droppable}
              onclick={() => onremove(i)}
              tabindex={selected === i ? 0 : -1}
            >
              <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
                <path d="M4.5 4.5l7 7M11.5 4.5l-7 7" stroke="currentColor" stroke-width="1.3" />
              </svg>
            </IconButton>
          </li>
        {/each}
        {#if dropAt?.line === line && dropAt.slot === card.items.length}<li class="drop" aria-hidden="true"></li>{/if}
        {#if card.items.length === 0}<li class="empty">{t("statusline-line-empty")}</li>{/if}
      </ul>
      <button
        type="button"
        class="card-add"
        disabled={full}
        aria-label={t("statusline-add-to-line", { n: line + 1 })}
        onclick={() => onadd(line)}
      >
        <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
          <path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="1.4" />
        </svg>
        {t("statusline-add")}
      </button>
    </section>
  {/each}
</div>

<button type="button" class="add-line" disabled={full} onclick={addLine}>
  <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
    <path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="1.4" />
  </svg>
  {t("statusline-line-add")}
</button>
<p class="sr-only" aria-live="polite">{announce}</p>

<style>
  .cards {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-2);
    border: 1px solid var(--stroke);
    border-radius: var(--radius-card);
    background: var(--surface-1);
  }
  .card header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-left: var(--space-1);
  }
  h3 {
    font: var(--type-caption);
    font-weight: 600;
    color: var(--text-2);
  }
  ul {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  li[data-row] {
    display: grid;
    grid-template-columns: 20px minmax(0, 1fr) auto auto;
    align-items: center;
    min-height: 36px;
    padding-right: var(--space-1);
    border-radius: var(--radius-control);
    border: 1px solid transparent;
  }
  /* A tint of the text color: the surfaces are white on white in the light theme. */
  li.hovered {
    background: color-mix(in srgb, var(--text-1) 7%, transparent);
  }
  li.selected {
    background: var(--surface-2);
    border-color: var(--stroke);
  }
  li.lifted {
    opacity: 0.6;
  }
  .drop {
    height: 2px;
    margin: -1px var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--accent);
  }
  .empty {
    display: grid;
    place-items: center;
    min-height: 36px;
    padding: 0 var(--space-2);
    border: 1px dashed var(--stroke);
    border-radius: var(--radius-control);
    font: var(--type-caption);
    color: var(--text-3);
    text-align: center;
  }
  .grip {
    display: grid;
    place-items: center;
    height: 100%;
    padding: 0;
    border: 0;
    background: none;
    color: var(--text-3);
    cursor: grab;
    touch-action: none;
  }
  .dragging,
  .dragging .grip {
    cursor: grabbing;
  }
  .grip:hover {
    color: var(--text-1);
  }
  .pick {
    display: flex;
    align-items: center;
    overflow: hidden;
    gap: var(--space-2);
    min-width: 0;
    min-height: 34px;
    padding: 0 var(--space-1);
    border: 0;
    background: none;
    color: var(--text-1);
    font: var(--type-body);
    text-align: start;
    cursor: pointer;
  }
  .name {
    flex: none;
    white-space: nowrap;
  }
  .mark {
    flex: none;
    color: var(--text-2);
  }
  .text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: pre;
    font: 400 12px / 16px var(--font-term);
    color: var(--text-2);
  }
  .text.hint {
    font: var(--type-caption);
  }
  /* Layout items read as structure: thinner rows, quieter names than the values'. */
  li.layout {
    min-height: 28px;
  }
  li.layout .pick {
    min-height: 26px;
  }
  li.layout .name {
    color: var(--text-2);
    font: var(--type-caption);
  }
  li.off .name,
  li.off .text {
    color: var(--text-3);
  }
  li.selected .name {
    font-weight: 600;
  }
  .tag {
    flex: none;
    margin-left: auto;
    font: var(--type-micro);
    padding: 1px var(--space-2);
    border-radius: var(--radius-pill);
    border: 1px solid var(--stroke);
    color: var(--text-2);
  }
  .card-add,
  .add-line {
    display: inline-flex;
    align-items: center;
    align-self: flex-start;
    gap: var(--space-1);
    min-height: 28px;
    padding: 0 var(--space-2);
    border-radius: var(--radius-control);
    background: none;
    color: var(--text-1);
    font: var(--type-body);
    cursor: pointer;
  }
  .card-add {
    border: 0;
    color: var(--text-2);
  }
  .add-line {
    margin-top: var(--space-2);
    border: 1px dashed var(--stroke);
  }
  .card-add:hover:not(:disabled),
  .add-line:hover:not(:disabled) {
    background: var(--surface-2);
    color: var(--text-1);
  }
  .card-add:disabled,
  .add-line:disabled {
    cursor: default;
    color: var(--text-3);
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  @media (forced-colors: active) {
    li.selected {
      border-color: Highlight;
    }
    .drop {
      background: Highlight;
    }
  }
</style>

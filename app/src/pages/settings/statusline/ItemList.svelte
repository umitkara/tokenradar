<!--
  The line's items in order. A row selects its item; the grip drags it (a static line marks
  where it lands, ADR-019) or moves it with ↑/↓; Alt+↑/↓ on a row moves it too, and ↑/↓ alone
  walks the rows. Every move is announced for screen readers. A line break is drawn as a rule.
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
import { canDrop, dropIndex, itemText, kindKey } from "../view";

interface Props {
  items: Item[];
  catalog: StatuslineKind[];
  preview: StatuslinePreview;
  selected: number | null;
  hovered: number | null;
  onselect: (index: number) => void;
  onhover: (index: number | null) => void;
  onmove: (from: number, to: number) => void;
  ontoggle: (index: number) => void;
  onremove: (index: number) => void;
  onadd: () => void;
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
  ontoggle,
  onremove,
  onadd,
}: Props = $props();

let list: HTMLUListElement;
let drag = $state<{ from: number; slot: number } | null>(null);
let announce = $state("");

const name = (kind: SegmentKind) => t(kindKey(kind));
const sampleOf = (kind: SegmentKind) => catalog.find((k) => k.kind === kind)?.sample ?? "";

/** The gap the drag would drop into, or `null` when it would not move anything. */
const dropAt = $derived(drag && dropIndex(drag.from, drag.slot) !== drag.from ? drag.slot : null);

function rowText(item: Item, index: number): string {
  if (item.kind === "line_break") return "";
  if (!item.enabled) return t("statusline-item-hidden");
  if (preview.absent.includes(item.kind)) return sampleOf(item.kind);
  return itemText(preview, index) ?? sampleOf(item.kind);
}

async function focusRow(index: number, part: "pick" | "grip" = "pick") {
  await tick();
  list.querySelector<HTMLElement>(`[data-row="${index}"] .${part}`)?.focus();
}

async function move(from: number, to: number, part: "pick" | "grip") {
  const item = items[from];
  if (!item || to < 0 || to >= items.length || to === from) return;
  onmove(from, to);
  announce = t("statusline-moved", {
    name: name(item.kind),
    position: to + 1,
    count: items.length,
  });
  await focusRow(to, part);
}

function rowKey(e: KeyboardEvent, index: number) {
  const by = e.key === "ArrowUp" ? -1 : e.key === "ArrowDown" ? 1 : 0;
  if (by !== 0) {
    e.preventDefault();
    if (e.altKey) void move(index, index + by, "pick");
    else if (items[index + by]) {
      onselect(index + by);
      void focusRow(index + by);
    }
  } else if (e.key === "Delete" && canDrop(items, index)) {
    e.preventDefault();
    onremove(index);
    void focusRow(Math.min(index, items.length - 2));
  }
}

function gripKey(e: KeyboardEvent, index: number) {
  const by = e.key === "ArrowUp" ? -1 : e.key === "ArrowDown" ? 1 : 0;
  if (by === 0) return;
  e.preventDefault();
  void move(index, index + by, "grip");
}

function slotAt(y: number): number {
  const rows = list.querySelectorAll<HTMLElement>("[data-row]");
  for (const row of rows) {
    const r = row.getBoundingClientRect();
    if (y < r.top + r.height / 2) return Number(row.dataset.row);
  }
  return items.length;
}

function down(e: PointerEvent, index: number) {
  if (e.button !== 0) return;
  (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  drag = { from: index, slot: index };
  onselect(index);
}

function moveDrag(e: PointerEvent) {
  if (drag) drag = { ...drag, slot: slotAt(e.clientY) };
}

function up() {
  if (!drag) return;
  const { from, slot } = drag;
  drag = null;
  const to = dropIndex(from, slot);
  if (to !== from) void move(from, to, "grip");
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

<ul class="items" bind:this={list} aria-label={t("statusline-items")} class:dragging={!!drag}>
  {#each items as item, i (i)}
    {@const absent = preview.absent.includes(item.kind)}
    {@const droppable = canDrop(items, i)}
    {#if dropAt === i}<li class="drop" aria-hidden="true"></li>{/if}
    <li
      data-row={i}
      class:selected={selected === i}
      class:hovered={hovered === i}
      class:off={!item.enabled || absent}
      class:break={item.kind === "line_break"}
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
        tabindex={selected === i || (selected === null && i === 0) ? 0 : -1}
        onclick={() => onselect(i)}
        onkeydown={(e) => rowKey(e, i)}
      >
        {#if item.kind === "line_break"}
          <span class="rule" aria-hidden="true"></span>
          <span class="name">{name(item.kind)}</span>
          <span class="rule" aria-hidden="true"></span>
        {:else}
          <span class="name">{name(item.kind)}</span>
          <span class="text">{rowText(item, i)}</span>
        {/if}
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
  {#if dropAt === items.length}<li class="drop" aria-hidden="true"></li>{/if}
</ul>

<button type="button" class="add" onclick={onadd}>
  <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
    <path d="M8 3v10M3 8h10" stroke="currentColor" stroke-width="1.4" />
  </svg>
  {t("statusline-add")}
</button>
<p class="sr-only" aria-live="polite">{announce}</p>

<style>
  .items {
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
  .text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: pre;
    font: 400 12px / 16px var(--font-term);
    color: var(--text-2);
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
  /* A line break: a rule across the row with its name in the middle. */
  li.break .pick {
    align-items: center;
  }
  li.break .name {
    font: var(--type-caption);
    color: var(--text-2);
  }
  .rule {
    flex: 1;
    border-top: 1px dashed var(--text-3);
  }
  .add {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    margin-top: var(--space-2);
    min-height: 28px;
    padding: 0 var(--space-2);
    border: 1px dashed var(--stroke);
    border-radius: var(--radius-control);
    background: none;
    color: var(--text-1);
    font: var(--type-body);
    cursor: pointer;
  }
  .add:hover {
    background: var(--surface-1);
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

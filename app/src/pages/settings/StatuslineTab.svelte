<!--
  The statusline editor (Steps 12j, 12l): a toolbar for the whole line, a terminal preview that
  stays in view, the items in one card per line and the selected item's settings. Every change goes into the draft,
  is drawn by the core at once and saved after a short pause; Undo walks back through this
  window's changes. Installing the bridge stays in Data sources.
-->
<script lang="ts">
import { onMount, untrack } from "svelte";
import Button from "../../lib/components/Button.svelte";
import IconButton from "../../lib/components/IconButton.svelte";
import Segmented from "../../lib/components/Segmented.svelte";
import Select from "../../lib/components/Select.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import {
  commands,
  type Item,
  type SegmentKind,
  type StatuslinePrefs,
  type TerminalFontView,
} from "../../lib/ipc/bindings";
import { inTauri } from "../../lib/ipc/runtime";
import { store } from "./state.svelte";
import AddPalette from "./statusline/AddPalette.svelte";
import { StatuslineEditor } from "./statusline/editor.svelte";
import { canDraw, previewFont } from "./statusline/font";
import ImportDialog from "./statusline/ImportDialog.svelte";
import ItemEditor from "./statusline/ItemEditor.svelte";
import ItemList from "./statusline/ItemList.svelte";
import PresetDialog from "./statusline/PresetDialog.svelte";
import TerminalFontDialog from "./statusline/TerminalFontDialog.svelte";
import TermPreview from "./statusline/TermPreview.svelte";
import {
  addLine,
  addToLine,
  applySeparator,
  COLOR_MODES,
  colorModeKey,
  dropInto,
  flexHintKey,
  fontNoteKey,
  ICON_SETS,
  iconSetKey,
  lines,
  MARGINS,
  moveItem,
  RESERVED_COLUMNS,
  removeItem,
  removeLine,
  SAMPLE_STATES,
  selectionAfterRemove,
  stateKey,
  type Tab,
  THEMES,
  themeKey,
  toggleHidden,
  updateItem,
  WIDTHS,
} from "./view";

interface Props {
  onerror: (key: string | null) => void;
  onnavigate: (tab: Tab, anchor: string | null) => void;
}

let { onerror, onnavigate }: Props = $props();

const editor = new StatuslineEditor();
// Inside `{#if draft}`, so the bindings come and go.
let palette = $state<AddPalette>();
let presetDialog = $state<PresetDialog>();
let importDialog = $state<ImportDialog>();
let fontDialog = $state<TerminalFontDialog>();
/** The terminal font state (Step 12o); `null` until loaded. */
let fontView = $state<TerminalFontView | null>(null);
let copied = $state<string | null>(null);
/** The card the palette adds to. */
let paletteLine = $state(0);
let copiedTimer: ReturnType<typeof setTimeout> | undefined;

const draft = $derived(editor.draft);
const items = $derived(draft?.items ?? []);
const selected = $derived(editor.selected);
const item = $derived(selected === null ? undefined : items[selected]);
const bridge = $derived(store.sources?.statusline.state ?? null);

const themes = $derived(THEMES.map((v) => ({ value: v, label: t(themeKey(v)) })));
const colorModes = $derived(COLOR_MODES.map((v) => ({ value: v, label: t(colorModeKey(v)) })));
const iconSets = $derived(ICON_SETS.map((v) => ({ value: v, label: t(iconSetKey(v)) })));
/** Only a change of the set reloads the catalog, not every edit of the draft. */
const icons = $derived(draft?.icons ?? "text");
const states = $derived(SAMPLE_STATES.map((v) => ({ value: v, label: t(stateKey(v)) })));
const widths = $derived(
  WIDTHS.map((v) => ({ value: v, label: t("statusline-columns", { n: v }) })),
);
const margins = $derived(
  MARGINS.map((v) => ({ value: v, label: t("statusline-columns", { n: v }) })),
);
const nerdFont = $derived(fontView?.font ?? null);
/** The preview draws with the installed Nerd Font, as the terminal will. */
const termFont = $derived(previewFont(nerdFont));
/** A font installed while TokenRadar runs reaches the preview only after a restart. */
const drawsFont = $derived(nerdFont ? canDraw(nerdFont.family) : false);
/** What the installed font means for the Nerd icons and for the capsule (Step 12p). */
const iconNote = $derived(fontNoteKey("icons", nerdFont, fontView !== null));
const capsuleNote = $derived(fontNoteKey("capsule", nerdFont, fontView !== null));
/** Whether the alignment note is worth showing: some line has a flexible space. */
const flexed = $derived(items.some((i) => i.kind === "flex" && i.enabled));

// The stored prefs become the draft unless the user's own changes are on their way.
$effect(() => {
  const stored = store.prefs?.statusline ?? null;
  untrack(() => editor.adopt(stored));
});

// The catalog's labels and samples are in the UI language and the icon set.
$effect(() => {
  void t("statusline-title");
  void editor.load(icons);
});

// Redrawn on every change of the draft, the sample state, the width or the language.
$effect(() => {
  void t("statusline-preview");
  void editor.draft;
  void editor.sample;
  void editor.width;
  void editor.redraw();
});

$effect(() => {
  onerror(editor.saveFailed ? "settings-save-failed" : null);
});

onMount(() => {
  if (inTauri()) void commands.getTerminalFont().then((v) => (fontView = v));
  const hidden = () => {
    if (document.visibilityState === "hidden") void editor.flush();
  };
  document.addEventListener("visibilitychange", hidden);
  return () => {
    document.removeEventListener("visibilitychange", hidden);
    clearTimeout(copiedTimer);
    void editor.flush();
  };
});

function setItems(next: Item[], key: string | null = null) {
  if (draft) editor.change({ ...draft, items: next }, key);
}

function set(patch: Partial<StatuslinePrefs>, key: string | null = null) {
  if (draft) editor.change({ ...draft, ...patch }, key);
}

function move(from: number, to: number) {
  setItems(moveItem(items, from, to));
  editor.selected = to;
}

function remove(index: number) {
  const before = items.length;
  const next = removeItem(items, index);
  if (next.length === before) return;
  setItems(next);
  editor.selected = selectionAfterRemove(before, index);
}

function add(kind: SegmentKind) {
  const added = addToLine(items, kind, paletteLine, selected, draft?.separator);
  if (!added) return;
  setItems(added.items);
  editor.selected = added.index;
}

function openPalette(line: number) {
  paletteLine = line;
  palette?.open();
}

function drop(from: number, line: number, slot: number): number | null {
  const moved = dropInto(items, from, line, slot);
  if (!moved) return null;
  setItems(moved.items);
  editor.selected = moved.index;
  return moved.index;
}

function newLine() {
  const next = addLine(items);
  if (next) setItems(next);
}

function dropLine(line: number) {
  const removed = removeLine(items, line);
  if (!removed) return;
  setItems(removed.items);
  const { start, count } = removed;
  if (selected === null || selected < start) return;
  // The selection went with the card: the next line's first item, else the previous line's.
  if (selected >= start + count) editor.selected = selected - count;
  else {
    const all = lines(removed.items);
    const near = all[line]?.items[0] ?? all[line - 1]?.items.at(-1);
    editor.selected = near ?? null;
  }
}

function replace(prefs: StatuslinePrefs) {
  editor.change(prefs);
  editor.selected = prefs.items.length > 0 ? 0 : null;
}

async function exportLine() {
  if (!draft) return;
  const text = await commands.exportStatusline(draft);
  try {
    await navigator.clipboard.writeText(text);
    copied = t("statusline-exported");
  } catch {
    copied = t("statusline-export-failed");
  }
  clearTimeout(copiedTimer);
  copiedTimer = setTimeout(() => (copied = null), 4000);
}

function keydown(e: KeyboardEvent) {
  if (!(e.ctrlKey && !e.shiftKey && !e.altKey && e.key.toLowerCase() === "z")) return;
  const target = e.target as HTMLElement | null;
  // Text fields keep their own undo.
  if (target?.closest("input, textarea")) return;
  e.preventDefault();
  editor.undo();
}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- The terminal font reaches every terminal sample on the page: preview, list, pickers, dialogs. -->
<div class="page" onkeydown={keydown} style:--font-term={termFont ?? undefined}>
  <header>
    <div class="title">
      <h2>{t("statusline-editor-title")}</h2>
      <p class="caption" role="status">{copied ?? t("statusline-autosave")}</p>
      <span class="spacer"></span>
      <Button disabled={!editor.canUndo} onclick={() => editor.undo()} title={t("statusline-undo-hint")}>
        {t("statusline-undo")}
      </Button>
      <!-- Both open the preset dialog; here they leave the toolbar room for the icon set. -->
      <Button disabled={!draft} onclick={() => presetDialog?.open("preset")}>{t("statusline-presets")}</Button>
      <Button disabled={!draft} onclick={() => presetDialog?.open("reset")}>{t("statusline-restore-default")}</Button>
    </div>
    {#if bridge === "not_installed" || bridge === "no_directory"}
      <p class="note">
        {t("statusline-editor-not-installed")}
        <button type="button" class="link" onclick={() => onnavigate("sources", "statusline")}>
          {t("statusline-editor-to-sources")}
        </button>
      </p>
    {/if}
    {#if draft}
      <div class="toolbar">
        <Segmented
          label={t("statusline-theme")}
          value={draft.theme}
          options={themes}
          onchange={(theme) => set({ theme })}
        />
        <Segmented
          label={t("statusline-colors")}
          value={draft.color}
          options={colorModes}
          onchange={(color) => set({ color })}
        />
        <Segmented
          label={t("statusline-icons")}
          value={draft.icons}
          options={iconSets}
          onchange={(icons) => set({ icons })}
        />
        <span class="spacer"></span>
        <!-- Icons, so that the toolbar stays on one line in Turkish too (Step 12p). -->
        <IconButton label={t("statusline-export")} onclick={exportLine}>
          <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
            <path d="M8 10V2.5M5 5.5l3-3 3 3M3 9.5v4h10v-4" fill="none" stroke="currentColor" stroke-width="1.2" />
          </svg>
        </IconButton>
        <IconButton label={t("statusline-import")} onclick={() => importDialog?.open()}>
          <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
            <path d="M8 2.5V10M5 7l3 3 3-3M3 9.5v4h10v-4" fill="none" stroke="currentColor" stroke-width="1.2" />
          </svg>
        </IconButton>
      </div>
      {#if draft.icons === "nerd" && iconNote}
        <p class="note">
          {t(iconNote)}
          <button type="button" class="link" onclick={() => fontDialog?.open()}>{t("font-open")}</button>
        </p>
      {:else if draft.theme !== "plain" && draft.icons !== "nerd" && nerdFont}
        <p class="caption">
          {t("statusline-icons-suggest")}
          <button type="button" class="link" onclick={() => set({ icons: "nerd" })}>
            {t("statusline-icons-use-nerd")}
          </button>
        </p>
      {/if}
    {/if}
  </header>

  {#if draft}
    <section class="preview" aria-label={t("statusline-preview")}>
      <div class="preview-bar">
        <Segmented
          label={t("statusline-state")}
          value={editor.sample}
          options={states}
          onchange={(s) => (editor.sample = s)}
        />
        <Segmented
          label={t("statusline-width")}
          value={editor.width}
          options={widths}
          onchange={(w) => (editor.width = w)}
        />
        <!-- Saved with the line, but next to the width: it narrows what the line may fill. -->
        <label class="margin" title={t("statusline-margin-hint")}>
          <span>{t("statusline-margin")}</span>
          <Select
            value={draft.margin}
            options={margins}
            onchange={(margin) => set({ margin })}
          />
        </label>
      </div>
      <TermPreview
        lines={editor.preview.lines}
        label={t("statusline-preview")}
        columns={editor.width}
        reserved={RESERVED_COLUMNS + draft.margin}
        reservedLabel={t("statusline-reserved", { n: RESERVED_COLUMNS + draft.margin })}
        selected={editor.selected}
        hovered={editor.hovered}
        onpick={(i) => (editor.selected = i)}
        onhover={(i) => (editor.hovered = i)}
      />
      {#if flexed}<p class="caption">{t("statusline-align-note")}</p>{/if}
      <!-- The font helper lives with the preview it is about, reachable in every theme. -->
      {#if fontView && !nerdFont && draft.theme !== "plain"}
        <p class="note">
          {t("statusline-font-note")}
          <button type="button" class="link" onclick={() => fontDialog?.open()}>{t("font-open")}</button>
        </p>
      {:else}
        <p class="caption">
          {#if nerdFont}
            {t(drawsFont ? "font-preview-with" : "font-preview-restart", { family: nerdFont.family })}
          {/if}
          <button type="button" class="link" onclick={() => fontDialog?.open()}>{t("font-open")}</button>
        </p>
      {/if}
    </section>

    <div class="columns">
      <div class="col">
        <ItemList
          {items}
          catalog={editor.catalog}
          preview={editor.preview}
          theme={draft.theme}
          {selected}
          hovered={editor.hovered}
          onselect={(i) => (editor.selected = i)}
          onhover={(i) => (editor.hovered = i)}
          onmove={move}
          ondrop={drop}
          ontoggle={(i) => setItems(toggleHidden(items, i))}
          onremove={remove}
          onadd={openPalette}
          onaddline={newLine}
          onremoveline={dropLine}
        />
      </div>
      <div class="col">
        {#if item && selected !== null}
          {@const index = selected}
          <ItemEditor
            {item}
            {index}
            count={items.length}
            info={editor.kind(item.kind)}
            absent={editor.preview.absent.includes(item.kind)}
            theme={draft.theme}
            {capsuleNote}
            flexHint={item.kind === "flex" ? flexHintKey(items, index) : null}
            onchange={(patch, key) => setItems(updateItem(items, index, patch), key)}
            onmove={(by) => move(index, index + by)}
            onapplyall={(glyph) => draft && editor.change(applySeparator(draft, glyph))}
          />
        {:else}
          <p class="caption">{t("statusline-select-hint")}</p>
        {/if}
      </div>
    </div>

    <AddPalette
      bind:this={palette}
      catalog={editor.catalog}
      {items}
      title={t("statusline-add-to-line", { n: paletteLine + 1 })}
      onadd={add}
    />
    <PresetDialog bind:this={presetDialog} presets={editor.presets} icons={draft.icons} onapply={replace} />
    <ImportDialog bind:this={importDialog} onapply={replace} />
    <TerminalFontDialog bind:this={fontDialog} onchange={(v) => (fontView = v)} />
  {/if}
</div>

<style>
  .page {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    container-type: inline-size;
  }
  header {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .title,
  .toolbar,
  .preview-bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }
  h2 {
    font: var(--type-title);
  }
  .spacer {
    flex: 1;
  }
  .caption {
    font: var(--type-caption);
    color: var(--text-2);
  }
  .note {
    font: var(--type-caption);
    color: var(--text-1);
    border-left: 2px solid var(--signal-warn);
    padding-left: var(--space-2);
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    font: inherit;
    text-decoration: underline;
    cursor: pointer;
  }
  .margin {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--text-2);
  }
  /* "0 col" needs no more than this; the shared drop-down is sized for sentences. */
  .margin :global(select) {
    min-width: 0;
  }
  .preview {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .columns {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: var(--space-4);
  }
  /* Positioned: the list's screen-reader announcement stays inside the scrolled column. */
  .col {
    position: relative;
    min-height: 0;
    overflow-x: hidden;
    overflow-y: auto;
    padding: 2px;
  }
  .col + .col {
    padding-left: var(--space-4);
    border-left: 1px solid var(--stroke);
  }
  /* Narrow window: one column, scrolled as a whole. */
  @container (max-width: 520px) {
    .columns {
      grid-template-columns: minmax(0, 1fr);
      overflow-y: auto;
    }
    .col {
      overflow: visible;
    }
    .col + .col {
      padding-left: 2px;
      border-left: 0;
      padding-top: var(--space-3);
      border-top: 1px solid var(--stroke);
    }
  }
</style>

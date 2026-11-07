<!--
  The statusline editor (Step 12j): a toolbar for the whole line, a terminal preview that stays
  in view, the item list and the selected item's settings. Every change goes into the draft,
  is drawn by the core at once and saved after a short pause; Undo walks back through this
  window's changes. Installing the bridge stays in Data sources.
-->
<script lang="ts">
import { onMount, untrack } from "svelte";
import Button from "../../lib/components/Button.svelte";
import Segmented from "../../lib/components/Segmented.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import {
  commands,
  type Item,
  type SegmentKind,
  type StatuslinePrefs,
} from "../../lib/ipc/bindings";
import { store } from "./state.svelte";
import AddPalette from "./statusline/AddPalette.svelte";
import { StatuslineEditor } from "./statusline/editor.svelte";
import ImportDialog from "./statusline/ImportDialog.svelte";
import ItemEditor from "./statusline/ItemEditor.svelte";
import ItemList from "./statusline/ItemList.svelte";
import PresetDialog from "./statusline/PresetDialog.svelte";
import TermPreview from "./statusline/TermPreview.svelte";
import {
  addItem,
  COLOR_MODES,
  colorModeKey,
  MAX_SEPARATOR,
  moveItem,
  removeItem,
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
let copied = $state<string | null>(null);
let copiedTimer: ReturnType<typeof setTimeout> | undefined;

const draft = $derived(editor.draft);
const items = $derived(draft?.items ?? []);
const selected = $derived(editor.selected);
const item = $derived(selected === null ? undefined : items[selected]);
const bridge = $derived(store.sources?.statusline.state ?? null);

const themes = $derived(THEMES.map((v) => ({ value: v, label: t(themeKey(v)) })));
const colorModes = $derived(COLOR_MODES.map((v) => ({ value: v, label: t(colorModeKey(v)) })));
const states = $derived(SAMPLE_STATES.map((v) => ({ value: v, label: t(stateKey(v)) })));
const widths = $derived(
  WIDTHS.map((v) => ({ value: v, label: t("statusline-columns", { n: v }) })),
);

// The stored prefs become the draft unless the user's own changes are on their way.
$effect(() => {
  const stored = store.prefs?.statusline ?? null;
  untrack(() => editor.adopt(stored));
});

// The catalog's labels and samples are in the UI language.
$effect(() => {
  void t("statusline-title");
  void editor.load();
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
  const added = addItem(items, kind, selected);
  if (!added) return;
  setItems(added.items);
  editor.selected = added.index;
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
<div class="page" onkeydown={keydown}>
  <header>
    <div class="title">
      <h2>{t("statusline-editor-title")}</h2>
      <p class="caption" role="status">{copied ?? t("statusline-autosave")}</p>
      <span class="spacer"></span>
      <Button disabled={!editor.canUndo} onclick={() => editor.undo()} title={t("statusline-undo-hint")}>
        {t("statusline-undo")}
      </Button>
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
        {#if draft.theme === "plain"}
          <label class="sep">
            <span>{t("statusline-separator")}</span>
            <input
              type="text"
              maxlength={MAX_SEPARATOR}
              placeholder="␠"
              value={draft.separator}
              oninput={(e) => set({ separator: e.currentTarget.value }, "separator")}
            />
          </label>
        {/if}
        <span class="spacer"></span>
        <Button onclick={() => presetDialog?.open("preset")}>{t("statusline-presets")}</Button>
        <Button onclick={exportLine}>{t("statusline-export")}</Button>
        <Button onclick={() => importDialog?.open()}>{t("statusline-import")}</Button>
      </div>
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
      </div>
      <TermPreview
        lines={editor.preview.lines}
        label={t("statusline-preview")}
        columns={editor.width}
        selected={editor.selected}
        hovered={editor.hovered}
        onpick={(i) => (editor.selected = i)}
        onhover={(i) => (editor.hovered = i)}
      />
      {#if draft.theme !== "plain"}
        <p class="caption">{t("statusline-font-note")}</p>
      {/if}
    </section>

    <div class="columns">
      <div class="col">
        <ItemList
          {items}
          catalog={editor.catalog}
          preview={editor.preview}
          {selected}
          hovered={editor.hovered}
          onselect={(i) => (editor.selected = i)}
          onhover={(i) => (editor.hovered = i)}
          onmove={move}
          ontoggle={(i) => setItems(toggleHidden(items, i))}
          onremove={remove}
          onadd={() => palette?.open()}
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
            onchange={(patch, key) => setItems(updateItem(items, index, patch), key)}
            onmove={(by) => move(index, index + by)}
          />
        {:else}
          <p class="caption">{t("statusline-select-hint")}</p>
        {/if}
      </div>
    </div>

    <AddPalette bind:this={palette} catalog={editor.catalog} {items} onadd={add} />
    <PresetDialog bind:this={presetDialog} presets={editor.presets} onapply={replace} />
    <ImportDialog bind:this={importDialog} onapply={replace} />
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
  .sep {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    color: var(--text-2);
  }
  .sep input {
    width: 44px;
    min-height: 28px;
    padding: 0 var(--space-1);
    text-align: center;
    border-radius: var(--radius-control);
    border: 1px solid var(--stroke);
    background: var(--surface-2);
    color: var(--text-1);
    font: 400 13px / 18px var(--font-term);
  }
  .sep input::placeholder {
    color: var(--text-3);
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
  .col {
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

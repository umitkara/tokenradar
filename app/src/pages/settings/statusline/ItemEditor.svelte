<!--
  The selected item's settings. What it offers comes from the core's catalog (`formats`,
  `labeled`, `colored`); a change goes up as a patch with a key, so quick edits of one field
  are undone together.
-->
<script lang="ts">
import Button from "../../../lib/components/Button.svelte";
import IconButton from "../../../lib/components/IconButton.svelte";
import Segmented from "../../../lib/components/Segmented.svelte";
import Toggle from "../../../lib/components/Toggle.svelte";
import { t } from "../../../lib/i18n/i18n.svelte";
import type { AnsiColor, Format, Item, StatuslineKind, Theme } from "../../../lib/ipc/bindings";
import {
  ANSI_COLORS,
  BAR_WIDTHS,
  barStyleKey,
  colorKey,
  formatKey,
  glyphRuns,
  groupKey,
  hasThresholdColors,
  isWindow,
  kindHintKey,
  kindKey,
  MAX_LABEL,
  MAX_SEPARATOR,
  MAX_TEXT,
  resetFormatKey,
  SPACE_WIDTHS,
} from "../view";

interface Props {
  item: Item;
  index: number;
  count: number;
  info: StatuslineKind | undefined;
  absent: boolean;
  /** A flexible space's effect in its line (`flexHintKey`); `null` on other kinds. */
  flexHint: string | null;
  /** The line's theme: Powerline blocks draw only the capsule bar. */
  theme: Theme;
  /** What the installed font means for the capsule (`fontNoteKey`); `null`: nothing to say. */
  capsuleNote: string | null;
  onchange: (patch: Partial<Item>, key: string | null) => void;
  onmove: (by: -1 | 1) => void;
  /** Gives every separator, and the ones added later, this item's glyph. */
  onapplyall: (glyph: string) => void;
}

let {
  item,
  index,
  count,
  info,
  absent,
  flexHint,
  theme,
  capsuleNote,
  onchange,
  onmove,
  onapplyall,
}: Props = $props();

const formats = $derived(
  (info?.formats ?? []).map((f: Format) => ({ value: f, label: t(formatKey(f)) })),
);
const swatches = $derived<(AnsiColor | null)[]>([null, ...ANSI_COLORS]);
const widths = $derived(SPACE_WIDTHS.map((n) => ({ value: n, label: String(n) })));
const resetFormats = $derived(
  (info?.reset_formats ?? []).map((f) => ({ value: f, label: t(resetFormatKey(f)) })),
);
const barWidths = $derived(BAR_WIDTHS.map((n) => ({ value: n, label: String(n) })));
// Each style shows its glyphs, drawn by the core.
const barStyles = $derived(
  (info?.bar_styles ?? []).map((b) => ({
    value: b.style,
    label: t(barStyleKey(b.style)),
    glyphs: glyphRuns(b.sample),
  })),
);
const showsBar = $derived(barStyles.length > 0 && item.format === "bar");
/** A note under the style picker: the bar is left out of Powerline blocks, or the font lacks it. */
const barNote = $derived(
  theme !== "plain" && item.bar_style !== "capsule"
    ? "statusline-bar-in-blocks"
    : item.bar_style === "capsule"
      ? capsuleNote
      : null,
);

let swatchGroup = $state<HTMLDivElement>();

function swatchKey(e: KeyboardEvent) {
  const i = swatches.indexOf(item.color);
  const step =
    e.key === "ArrowRight" || e.key === "ArrowDown"
      ? 1
      : e.key === "ArrowLeft" || e.key === "ArrowUp"
        ? -1
        : 0;
  if (step === 0) return;
  e.preventDefault();
  const at = (i + step + swatches.length) % swatches.length;
  onchange({ color: swatches[at] ?? null }, null);
  swatchGroup?.querySelectorAll<HTMLButtonElement>("button")[at]?.focus();
}
</script>

<div class="editor">
  <header>
    <div>
      <h3>{t(kindKey(item.kind))}</h3>
      {#if info}<p class="caption">{t(groupKey(info.group))}</p>{/if}
    </div>
    <span class="order">
      <IconButton label={t("statusline-move-up")} disabled={index === 0} onclick={() => onmove(-1)}>
        <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
          <path d="M4 10l4-4 4 4" fill="none" stroke="currentColor" stroke-width="1.5" />
        </svg>
      </IconButton>
      <IconButton
        label={t("statusline-move-down")}
        disabled={index === count - 1}
        onclick={() => onmove(1)}
      >
        <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
          <path d="M4 6l4 4 4-4" fill="none" stroke="currentColor" stroke-width="1.5" />
        </svg>
      </IconButton>
    </span>
  </header>
  <p class="hint">{t(kindHintKey(item.kind))}</p>
  {#if absent}<p class="note">{t("statusline-absent-note")}</p>{/if}
  {#if !item.enabled}<p class="note">{t("statusline-hidden-note")}</p>{/if}

  {#if formats.length > 1}
    <div class="field">
      <span class="label">{t("statusline-format")}</span>
      <Segmented
        label={t("statusline-format")}
        value={item.format ?? info?.default_format ?? formats[0]?.value ?? "percent"}
        options={formats}
        onchange={(format) => onchange({ format }, null)}
      />
    </div>
  {/if}

  {#if showsBar}
    <div class="field">
      <span class="label">{t("statusline-bar-width")}</span>
      <Segmented
        label={t("statusline-bar-width")}
        value={item.bar_width}
        options={barWidths}
        onchange={(bar_width) => onchange({ bar_width }, null)}
      />
    </div>
    <div class="field">
      <span class="label">{t("statusline-bar-style")}</span>
      <Segmented
        label={t("statusline-bar-style")}
        value={item.bar_style}
        options={barStyles}
        onchange={(bar_style) => onchange({ bar_style }, null)}
      />
      {#if barNote}<p class="note">{t(barNote)}</p>{/if}
    </div>
  {/if}

  {#if info?.labeled}
    <div class="field">
      <label class="label" for="sl-label">{t("statusline-label")}</label>
      <input
        id="sl-label"
        type="text"
        maxlength={MAX_LABEL}
        placeholder={info.default_label || t("statusline-label-none")}
        value={item.label}
        oninput={(e) => onchange({ label: e.currentTarget.value }, `label:${index}`)}
      />
      <span class="caption">{t("statusline-label-hint")}</span>
    </div>
  {/if}

  {#if item.kind === "separator"}
    <div class="field">
      <label class="label" for="sl-glyph">{t("statusline-glyph")}</label>
      <input
        id="sl-glyph"
        class="short"
        type="text"
        maxlength={MAX_SEPARATOR}
        placeholder="│"
        value={item.text}
        oninput={(e) => onchange({ text: e.currentTarget.value }, `text:${index}`)}
      />
      <Button onclick={() => onapplyall(item.text.trim() === "" ? "│" : item.text)}>
        {t("statusline-apply-all")}
      </Button>
      <span class="caption">{t("statusline-apply-all-hint")}</span>
    </div>
  {:else if item.kind === "space"}
    <div class="field">
      <span class="label">{t("statusline-space-width")}</span>
      <Segmented
        label={t("statusline-space-width")}
        value={item.width}
        options={widths}
        onchange={(width) => onchange({ width }, null)}
      />
      <span class="caption">{t("statusline-columns", { n: item.width })}</span>
    </div>
  {:else if item.kind === "flex" && flexHint}
    <p class="note">{t(flexHint)}</p>
  {:else if item.kind === "custom_text"}
    <div class="field">
      <label class="label" for="sl-text">{t("statusline-text")}</label>
      <input
        id="sl-text"
        type="text"
        maxlength={MAX_TEXT}
        value={item.text}
        oninput={(e) => onchange({ text: e.currentTarget.value }, `text:${index}`)}
      />
    </div>
  {/if}

  {#if info?.colored}
    <div class="field">
      <span class="label" id="sl-color-label">{t("statusline-color")}</span>
      <div
        class="swatches"
        role="radiogroup"
        aria-labelledby="sl-color-label"
        tabindex="-1"
        bind:this={swatchGroup}
        onkeydown={swatchKey}
      >
        {#each swatches as color (color ?? "auto")}
          {@const on = item.color === color}
          {@const name = color ? t(colorKey(color)) : t("statusline-color-auto")}
          <button
            type="button"
            role="radio"
            aria-checked={on}
            aria-label={name}
            title={name}
            tabindex={on ? 0 : -1}
            class={color ? `swatch c-${color.replaceAll("_", "-")}` : "swatch auto"}
            onclick={() => onchange({ color }, null)}
          >
            {#if !color}{t("statusline-color-auto")}{/if}
          </button>
        {/each}
      </div>
      {#if hasThresholdColors(item.kind)}
        <span class="caption">{t("statusline-color-threshold-note")}</span>
      {/if}
    </div>
  {/if}

  {#if isWindow(item.kind)}
    <div class="switches">
      <Toggle
        checked={item.reset}
        label={t("statusline-reset-countdown")}
        onchange={(reset) => onchange({ reset }, null)}
      />
      {#if item.reset && resetFormats.length > 0}
        <div class="field">
          <span class="label">{t("statusline-reset-format")}</span>
          <Segmented
            label={t("statusline-reset-format")}
            value={item.reset_format}
            options={resetFormats}
            onchange={(reset_format) => onchange({ reset_format }, null)}
          />
        </div>
      {/if}
      <Toggle
        checked={item.pace}
        label={t("statusline-pace")}
        onchange={(pace) => onchange({ pace }, null)}
      />
      <span class="caption">{t("statusline-pace-hint")}</span>
    </div>
  {/if}
</div>

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-2);
  }
  h3 {
    font: var(--type-title);
  }
  .order {
    display: flex;
    gap: var(--space-1);
  }
  .hint {
    color: var(--text-2);
    margin-top: calc(-1 * var(--space-2));
  }
  .note {
    font: var(--type-caption);
    color: var(--text-1);
    border-left: 2px solid var(--text-3);
    padding-left: var(--space-2);
  }
  .field {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-1);
  }
  .label {
    font-weight: 600;
  }
  .caption {
    font: var(--type-caption);
    color: var(--text-2);
  }
  input {
    font: 400 13px / 18px var(--font-term);
    width: 100%;
    max-width: 240px;
    min-height: 28px;
    padding: 0 var(--space-2);
    border-radius: var(--radius-control);
    border: 1px solid var(--stroke);
    background: var(--surface-2);
    color: var(--text-1);
  }
  input.short {
    width: 64px;
  }
  input::placeholder {
    color: var(--text-3);
  }
  .swatches {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    padding: 6px;
    border-radius: var(--radius-control);
    background: var(--term-bg);
  }
  .swatch {
    width: 22px;
    height: 22px;
    padding: 0;
    border-radius: 3px;
    border: 1px solid #ffffff30;
    cursor: pointer;
  }
  .swatch[aria-checked="true"] {
    outline: 2px solid var(--term-fg);
    outline-offset: 1px;
  }
  .swatch.auto {
    width: auto;
    padding: 0 6px;
    background: transparent;
    color: var(--term-fg);
    font: var(--type-caption);
  }
  .switches {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
  }
  .c-black {
    background: var(--term-black);
  }
  .c-red {
    background: var(--term-red);
  }
  .c-green {
    background: var(--term-green);
  }
  .c-yellow {
    background: var(--term-yellow);
  }
  .c-blue {
    background: var(--term-blue);
  }
  .c-magenta {
    background: var(--term-magenta);
  }
  .c-cyan {
    background: var(--term-cyan);
  }
  .c-white {
    background: var(--term-white);
  }
  .c-gray {
    background: var(--term-gray);
  }
  .c-bright-red {
    background: var(--term-bright-red);
  }
  .c-bright-green {
    background: var(--term-bright-green);
  }
  .c-bright-yellow {
    background: var(--term-bright-yellow);
  }
  .c-bright-blue {
    background: var(--term-bright-blue);
  }
  .c-bright-magenta {
    background: var(--term-bright-magenta);
  }
  .c-bright-cyan {
    background: var(--term-bright-cyan);
  }
  .c-bright-white {
    background: var(--term-bright-white);
  }
  @media (forced-colors: active) {
    .swatches {
      forced-color-adjust: none;
    }
  }
</style>

<!--
  A segmented control: a few mutually exclusive options side by side, for choices too short for
  a drop-down. A radio group to assistive tech: arrow keys move the choice, Tab leaves the group.
-->
<script lang="ts" generics="T extends string | number">
import type { GlyphRun } from "./glyphs";

interface Option {
  value: T;
  label: string;
  /**
   * Terminal glyphs shown before the label in the terminal font, as text or as colored runs;
   * hidden from assistive tech.
   */
  glyphs?: string | readonly GlyphRun[];
}

interface Props {
  value: T;
  options: readonly Option[];
  /** The group's accessible name. */
  label: string;
  disabled?: boolean;
  onchange?: (value: T) => void;
}

let { value, options, label, disabled = false, onchange }: Props = $props();

let group: HTMLDivElement;

function pick(next: T) {
  if (next !== value) onchange?.(next);
}

function keydown(e: KeyboardEvent) {
  const i = options.findIndex((o) => o.value === value);
  const step =
    e.key === "ArrowRight" || e.key === "ArrowDown"
      ? 1
      : e.key === "ArrowLeft" || e.key === "ArrowUp"
        ? -1
        : 0;
  const at =
    e.key === "Home" ? 0 : e.key === "End" ? options.length - 1 : step !== 0 ? i + step : null;
  if (at === null) return;
  e.preventDefault();
  const next = options[(at + options.length) % options.length];
  if (!next) return;
  pick(next.value);
  const buttons = group.querySelectorAll<HTMLButtonElement>("button");
  buttons[(at + options.length) % options.length]?.focus();
}
</script>

<div class="segmented" role="radiogroup" aria-label={label} bind:this={group} onkeydown={keydown} tabindex="-1">
  {#each options as option (option.value)}
    {@const on = option.value === value}
    <button
      type="button"
      role="radio"
      aria-checked={on}
      tabindex={on ? 0 : -1}
      {disabled}
      onclick={() => pick(option.value)}
    >
      {#if typeof option.glyphs === "string" && option.glyphs}
        <span class="glyphs" aria-hidden="true">{option.glyphs}</span>
      {:else if typeof option.glyphs === "object" && option.glyphs.length > 0}
        <span class="glyphs" aria-hidden="true"
          >{#each option.glyphs as run, i (i)}<span
              style:color={run.color}
              style:background-color={run.background}>{run.text}</span
            >{/each}</span
        >
      {/if}
      {option.label}
    </button>
  {/each}
</div>

<style>
  .segmented {
    display: inline-flex;
    /* Options wrap rather than spill out of a narrow column (five bar styles, Step 12p). */
    flex-wrap: wrap;
    max-width: 100%;
    padding: 2px;
    gap: 2px;
    border-radius: var(--radius-control);
    border: 1px solid var(--stroke);
    background: var(--surface-1);
  }
  button {
    font: var(--type-body);
    min-height: 24px;
    padding: 0 var(--space-2);
    border: 0;
    border-radius: 3px;
    background: transparent;
    color: var(--text-2);
    cursor: pointer;
    white-space: nowrap;
    transition: background-color var(--dur-fast) var(--ease-out);
  }
  button:hover:not(:disabled):not([aria-checked="true"]) {
    background: var(--surface-2);
    color: var(--text-1);
  }
  button[aria-checked="true"] {
    background: var(--surface-2);
    color: var(--text-1);
    font-weight: 600;
    box-shadow: inset 0 -2px 0 var(--accent);
  }
  .glyphs {
    font-family: var(--font-term);
    margin-right: 4px;
    /* Terminal cells: a solid bar is colored spaces, which must not collapse. */
    white-space: pre;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  @media (forced-colors: active) {
    button[aria-checked="true"] {
      background: Highlight;
      color: HighlightText;
    }
  }
</style>

<!--
  Applying a preset, or going back to the default: each choice is drawn by the core before the
  user confirms, because it replaces every item, the theme and the colors. A preset keeps the
  user's icon set, which depends on their font like the theme does not (Step 12p); going back to
  the default resets it too. Undo still brings the previous line back.
-->
<script lang="ts">
import Button from "../../../lib/components/Button.svelte";
import { t } from "../../../lib/i18n/i18n.svelte";
import {
  commands,
  type IconSet,
  type Preset,
  type Span,
  type StatuslinePrefs,
  type StatuslinePreset,
} from "../../../lib/ipc/bindings";
import { inTauri } from "../../../lib/ipc/runtime";
import TermPreview from "./TermPreview.svelte";

interface Props {
  presets: StatuslinePreset[];
  /** The draft's icon set, kept by a preset. */
  icons: IconSet;
  onapply: (prefs: StatuslinePrefs) => void;
}

let { presets, icons, onapply }: Props = $props();

let dialog: HTMLDialogElement;
let mode = $state<"preset" | "reset">("preset");
let chosen = $state<Preset>("default");
let lines = $state<Partial<Record<Preset, Span[][]>>>({});

const shown = $derived(mode === "reset" ? presets.filter((p) => p.preset === "default") : presets);
const presetKey = (p: Preset) => `statusline-preset-${p.replaceAll("_", "-")}`;
/** What a choice applies: a preset with the user's icons, the default as it is. */
const applied = (p: StatuslinePreset): StatuslinePrefs =>
  mode === "reset" ? p.prefs : { ...p.prefs, icons };

export async function open(next: "preset" | "reset") {
  mode = next;
  chosen = "default";
  dialog.showModal();
  if (!inTauri()) return;
  const drawn = await Promise.all(
    presets.map(async (p) => {
      const preview = await commands.previewStatusline(applied(p), "normal", 120);
      return [p.preset, preview.lines] as const;
    }),
  );
  lines = Object.fromEntries(drawn);
}

function apply() {
  const preset = presets.find((p) => p.preset === chosen);
  dialog.close();
  if (preset) onapply(applied(preset));
}
</script>

<dialog bind:this={dialog} aria-labelledby="sl-preset-title">
  <h2 id="sl-preset-title">
    {t(mode === "reset" ? "statusline-default-title" : "statusline-presets-title")}
  </h2>
  <p class="intro">
    {t(mode === "reset" ? "statusline-default-intro" : "statusline-presets-intro")}
  </p>
  <div class="choices">
    {#each shown as p (p.preset)}
      <label class="choice" class:on={chosen === p.preset}>
        <span class="head">
          {#if mode === "preset"}
            <input type="radio" name="sl-preset" value={p.preset} bind:group={chosen} />
          {/if}
          <span class="name">{t(presetKey(p.preset))}</span>
          <span class="hint">{t(`${presetKey(p.preset)}-hint`)}</span>
        </span>
        <TermPreview lines={lines[p.preset] ?? []} label={t(presetKey(p.preset))} />
      </label>
    {/each}
  </div>
  {#if presets.some((p) => p.prefs.theme !== "plain") && mode === "preset"}
    <p class="note">{t("statusline-font-note")}</p>
  {/if}
  <div class="actions">
    <Button onclick={() => dialog.close()}>{t("settings-cancel")}</Button>
    <Button variant="primary" onclick={apply}>
      {t(mode === "reset" ? "statusline-default-confirm" : "statusline-presets-confirm")}
    </Button>
  </div>
</dialog>

<style>
  dialog {
    width: min(620px, calc(100vw - 48px));
    max-height: calc(100vh - 48px);
    padding: var(--space-4);
    border: 1px solid var(--stroke);
    border-radius: var(--radius-card);
    background: var(--surface-0-solid);
    color: var(--text-1);
  }
  dialog::backdrop {
    background: oklch(0 0 0 / 0.4);
  }
  h2 {
    font: var(--type-title);
  }
  .intro {
    color: var(--text-2);
    margin: var(--space-1) 0 var(--space-3);
  }
  .choices {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .choice {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-2);
    border: 1px solid var(--stroke);
    border-radius: var(--radius-control);
    background: var(--surface-1);
    cursor: pointer;
  }
  .choice.on {
    border-color: var(--accent);
  }
  .head {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
  }
  input {
    accent-color: var(--accent);
    align-self: center;
  }
  .name {
    font-weight: 600;
  }
  .hint,
  .note {
    font: var(--type-caption);
    color: var(--text-2);
  }
  .note {
    margin-top: var(--space-2);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-4);
  }
  @media (forced-colors: active) {
    .choice.on {
      border-color: Highlight;
    }
  }
</style>

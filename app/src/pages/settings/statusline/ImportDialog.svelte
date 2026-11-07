<!--
  Importing a shared `[statusline]` table: the core reads the text without saving it, the line
  is drawn with the result and skipped keys are listed; only "Apply" changes the draft.
-->
<script lang="ts">
import Button from "../../../lib/components/Button.svelte";
import { t } from "../../../lib/i18n/i18n.svelte";
import {
  commands,
  type Imported,
  type Span,
  type StatuslinePrefs,
} from "../../../lib/ipc/bindings";
import { inTauri } from "../../../lib/ipc/runtime";
import TermPreview from "./TermPreview.svelte";

interface Props {
  onapply: (prefs: StatuslinePrefs) => void;
}

let { onapply }: Props = $props();

let dialog: HTMLDialogElement;
let text = $state("");
let read = $state<Imported | null>(null);
let lines = $state<Span[][]>([]);
let error = $state<string | null>(null);

export function open() {
  text = "";
  read = null;
  lines = [];
  error = null;
  dialog.showModal();
}

async function check() {
  read = null;
  error = null;
  if (!inTauri()) return;
  const r = await commands.importStatusline(text);
  if (r.status === "error") {
    error = r.error;
    return;
  }
  read = r.data;
  lines = (await commands.previewStatusline(r.data.prefs, "normal", 120)).lines;
}

function apply() {
  const prefs = read?.prefs;
  dialog.close();
  if (prefs) onapply(prefs);
}
</script>

<dialog bind:this={dialog} aria-labelledby="sl-import-title">
  <h2 id="sl-import-title">{t("statusline-import-title")}</h2>
  <p class="intro">{t("statusline-import-intro")}</p>
  <textarea
    aria-label={t("statusline-import-title")}
    spellcheck="false"
    rows="8"
    placeholder={'[statusline]\ntheme = "plain"\n…'}
    bind:value={text}
    oninput={() => (read = null)}
  ></textarea>
  <div class="row">
    <Button disabled={text.trim() === ""} onclick={check}>{t("statusline-import-read")}</Button>
  </div>
  {#if error}
    <p class="error" role="alert">{t("statusline-import-error")}</p>
    <pre class="detail">{error}</pre>
  {/if}
  {#if read}
    <p class="caption">{t("statusline-import-preview")}</p>
    <TermPreview {lines} label={t("statusline-import-preview")} />
    {#if read.ignored.length > 0}
      <p class="caption">{t("statusline-import-ignored", { count: read.ignored.length })}</p>
      <ul class="ignored">
        {#each read.ignored as key (key)}<li><code>{key}</code></li>{/each}
      </ul>
    {/if}
  {/if}
  <div class="actions">
    <Button onclick={() => dialog.close()}>{t("settings-cancel")}</Button>
    <Button variant="primary" disabled={!read} onclick={apply}>{t("statusline-import-apply")}</Button>
  </div>
</dialog>

<style>
  dialog {
    width: min(560px, calc(100vw - 48px));
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
  textarea {
    width: 100%;
    resize: vertical;
    padding: var(--space-2);
    border-radius: var(--radius-control);
    border: 1px solid var(--stroke);
    background: var(--surface-2);
    color: var(--text-1);
    font: 400 12px / 18px var(--font-term);
  }
  .row {
    margin-top: var(--space-2);
  }
  .caption {
    font: var(--type-caption);
    color: var(--text-2);
    margin: var(--space-3) 0 var(--space-1);
  }
  .error {
    margin-top: var(--space-3);
    padding-left: var(--space-2);
    border-left: 2px solid var(--signal-crit);
  }
  .detail {
    margin-top: var(--space-1);
    font: 400 11px / 16px var(--font-term);
    color: var(--text-2);
    white-space: pre-wrap;
    user-select: text;
  }
  .ignored {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }
  code {
    font: 400 11px / 16px var(--font-term);
    padding: 0 var(--space-1);
    border-radius: 3px;
    background: var(--surface-2);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-4);
  }
</style>

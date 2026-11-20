<!--
  "Export…" on a list page (Step 17a): a small dialog picks the format (and for sessions whether
  project paths are left out), then the backend asks where to save and writes the file
  (`export_data`). The request is read when Save is pressed, so it always matches what the page
  shows. Daily totals are CSV only.
-->
<script lang="ts">
import Button from "../../lib/components/Button.svelte";
import Segmented from "../../lib/components/Segmented.svelte";
import Toggle from "../../lib/components/Toggle.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import { commands, type ExportFormat, type ExportRequest } from "../../lib/ipc/bindings";
import { inTauri } from "../../lib/ipc/runtime";

interface Props {
  kind: ExportRequest["kind"];
  request: () => ExportRequest;
}

let { kind, request }: Props = $props();

let dialog: HTMLDialogElement;
let format = $state<ExportFormat>("csv");
let hidePaths = $state(false);
let busy = $state(false);
let status = $state<{ ok: boolean; text: string } | null>(null);

const formats = [
  { value: "csv" as const, label: "CSV" },
  { value: "json" as const, label: "JSON" },
];

function openDialog() {
  status = null;
  dialog.showModal();
}

async function save() {
  if (!inTauri()) return;
  busy = true;
  const result = await commands.exportData(request(), kind === "daily" ? "csv" : format, hidePaths);
  busy = false;
  if (result.status === "error") {
    status = { ok: false, text: t("export-failed", { error: result.error }) };
  } else if (result.data != null) {
    status = { ok: true, text: t("export-saved", { path: result.data }) };
  }
}
</script>

<Button onclick={openDialog}>{t("export-button")}</Button>

<dialog bind:this={dialog} aria-labelledby="export-title-{kind}">
  <h2 id="export-title-{kind}">{t("export-title", { what: kind })}</h2>
  {#if kind !== "daily"}
    <div class="row">
      <span class="label">{t("export-format")}</span>
      <Segmented value={format} options={formats} label={t("export-format")} onchange={(f) => (format = f)} />
    </div>
  {/if}
  {#if kind === "sessions"}
    <Toggle bind:checked={hidePaths} label={t("export-hide-paths")} />
  {/if}
  <p class="note">{t("export-scope")} {t("export-privacy")}</p>
  {#if status}
    <p class="status" class:failed={!status.ok} role="status">{status.text}</p>
  {/if}
  <div class="actions">
    <Button disabled={busy} onclick={() => dialog.close()}>{t("window-close")}</Button>
    <Button variant="primary" disabled={busy} onclick={save}>{t("export-save")}</Button>
  </div>
</dialog>

<style>
  dialog {
    width: min(440px, calc(100vw - 48px));
    padding: var(--space-4);
    border: 1px solid var(--stroke);
    border-radius: var(--radius-card);
    background: var(--surface-0-solid);
    color: var(--text-1);
  }
  dialog[open] {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  dialog::backdrop {
    background: oklch(0 0 0 / 0.4);
  }
  h2 {
    margin: 0;
    font: var(--type-title);
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .label {
    font: var(--type-body);
    color: var(--text-2);
  }
  .note {
    margin: 0;
    font: var(--type-caption);
    color: var(--text-2);
  }
  .status {
    margin: 0;
    font: var(--type-caption);
    color: var(--text-1);
    overflow-wrap: anywhere;
    padding-left: var(--space-2);
    border-left: 2px solid var(--accent);
  }
  .status.failed {
    border-left-color: var(--signal-crit);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }
</style>

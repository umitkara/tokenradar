<!--
  Privacy & data: storage and retention, where the app connects, deleting TokenRadar's data
  (two confirmations: the dialog and its checkbox) and the diagnostics bundle (`#diagnostics`).
-->
<script lang="ts">
import Button from "../../lib/components/Button.svelte";
import Card from "../../lib/components/Card.svelte";
import Select from "../../lib/components/Select.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import { commands, type LogLevel } from "../../lib/ipc/bindings";
import Field from "./Field.svelte";
import { loadAll, savePrefs, store } from "./state.svelte";
import { optionsWith, RETENTION_PRESETS, sizeMessage } from "./view";

let { onerror }: { onerror: (key: string | null) => void } = $props();

let dialog: HTMLDialogElement;
let understood = $state(false);
let deleting = $state(false);
let deleteResult = $state<string | null>(null);
let bundleResult = $state<string | null>(null);
let creating = $state(false);

const LEVELS: readonly LogLevel[] = ["error", "warn", "info", "debug", "trace"];

const storage = $derived(store.storage);
const prefs = $derived(store.prefs);
const apiOn = $derived(store.sources?.api.granted ?? false);
const updatesOn = $derived(prefs?.data.update_check ?? true);
const retention = $derived(
  optionsWith(RETENTION_PRESETS, prefs?.data.raw_retention_days ?? 90).map((days) => ({
    value: days,
    label: t("privacy-retention-days", { days }),
  })),
);
const levels = $derived(LEVELS.map((value) => ({ value, label: t(`log-level-${value}`) })));

function size(bytes: number): string {
  const m = sizeMessage(bytes);
  return t(m.key, m.args);
}

async function setRetention(days: number) {
  if (!prefs) return;
  const data = { ...prefs.data, raw_retention_days: days };
  onerror((await savePrefs(() => commands.setDataPrefs(data))) ? null : "settings-save-failed");
}

async function setLevel(level: LogLevel) {
  onerror((await savePrefs(() => commands.setLogLevel(level))) ? null : "settings-save-failed");
}

function askDelete() {
  understood = false;
  deleteResult = null;
  dialog.showModal();
}

async function confirmDelete() {
  deleting = true;
  const r = await commands.deleteAllData();
  deleting = false;
  dialog.close();
  if (r.status === "error") console.error("settings: delete", r.error);
  deleteResult = t(r.status === "ok" ? "privacy-delete-done" : "privacy-delete-partial");
  await loadAll();
}

async function createBundle() {
  creating = true;
  bundleResult = null;
  const r = await commands.createDiagnostics();
  creating = false;
  if (r.status === "ok") {
    bundleResult = r.data ? t("privacy-diagnostics-done", { path: r.data }) : null;
  } else {
    console.error("settings: diagnostics", r.error);
    bundleResult = t("privacy-diagnostics-failed");
  }
}
</script>

<Card title={t("privacy-storage")}>
  {#if storage}
    <dl class="facts">
      <dt>{t("privacy-data-dir")}</dt>
      <dd><code>{storage.data_dir}</code></dd>
      <dt>{t("privacy-db")}</dt>
      <dd class="num">{size(storage.db_bytes)}</dd>
      <dt>{t("privacy-logs")}</dt>
      <dd class="num">{size(storage.logs_bytes)}</dd>
    </dl>
    {#if storage.portable}<p class="hint">{t("privacy-portable")}</p>{/if}
  {/if}
  {#if prefs}
    <Field label={t("privacy-retention")} id="retention" hint={t("privacy-retention-hint")}>
      <Select
        id="retention"
        value={prefs.data.raw_retention_days ?? 90}
        options={retention}
        onchange={setRetention}
      />
    </Field>
  {/if}
</Card>

<Card title={t("privacy-network")}>
  <p>{t("privacy-network-intro")}</p>
  <ul class="hosts">
    <li>
      <span>{t("privacy-network-api")}</span>
      <span class="state">{t(apiOn ? "privacy-network-on" : "privacy-network-off")}</span>
    </li>
    <li>
      <span>{t("privacy-network-github")}</span>
      <span class="state">{t(updatesOn ? "privacy-network-on" : "privacy-network-off")}</span>
    </li>
  </ul>
  <p class="hint">{t("privacy-network-none")}</p>
</Card>

<section id="diagnostics" tabindex="-1">
  <Card title={t("privacy-diagnostics")}>
    <p class="hint">{t("privacy-diagnostics-hint")}</p>
    <div class="actions">
      <Button disabled={creating} onclick={createBundle}>{t("privacy-diagnostics-button")}</Button>
    </div>
    {#if bundleResult}<p class="result" role="status"><code>{bundleResult}</code></p>{/if}
    {#if prefs}
      <Field label={t("privacy-log-level")} id="log-level">
        <Select id="log-level" value={prefs.log_level} options={levels} onchange={setLevel} />
      </Field>
    {/if}
  </Card>
</section>

<Card title={t("privacy-delete")}>
  <p class="hint">{t("privacy-delete-hint")}</p>
  <div class="actions">
    <Button onclick={askDelete}>{t("privacy-delete-button")}</Button>
  </div>
  {#if deleteResult}<p class="result" role="status">{deleteResult}</p>{/if}
</Card>

<dialog bind:this={dialog} aria-labelledby="delete-title">
  <h2 id="delete-title">{t("privacy-delete-title")}</h2>
  <p>{t("privacy-delete-what")}</p>
  <p>{t("privacy-delete-kept")}</p>
  <label class="check">
    <input type="checkbox" bind:checked={understood} disabled={deleting} />
    {t("privacy-delete-check")}
  </label>
  <div class="actions end">
    <Button disabled={deleting} onclick={() => dialog.close()}>{t("settings-cancel")}</Button>
    <Button variant="primary" disabled={!understood || deleting} onclick={confirmDelete}>
      {t("privacy-delete-confirm")}
    </Button>
  </div>
</dialog>

<style>
  .hint {
    font: var(--type-caption);
    color: var(--text-2);
    margin-top: var(--space-1);
  }
  .facts {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--space-1) var(--space-3);
    margin: 0;
  }
  .facts dd {
    margin: 0;
    min-width: 0;
  }
  code {
    font: 400 12px / 16px var(--font-mono);
    user-select: text;
    word-break: break-all;
  }
  .num {
    font: 400 12px / 18px var(--font-mono);
  }
  .hosts {
    margin: var(--space-2) 0;
  }
  .hosts li {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-1) 0;
  }
  .state {
    color: var(--text-2);
  }
  section {
    outline: none;
    scroll-margin-top: var(--space-3);
  }
  .actions {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
  .end {
    justify-content: flex-end;
  }
  .result {
    margin-top: var(--space-2);
    font: var(--type-caption);
    color: var(--text-2);
  }
  dialog {
    width: min(480px, calc(100vw - 48px));
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
    font: var(--type-title);
  }
  .check {
    display: flex;
    gap: var(--space-2);
    align-items: center;
  }
</style>

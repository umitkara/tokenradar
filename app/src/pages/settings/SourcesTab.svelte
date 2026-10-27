<!--
  Data sources: the Claude Code folders, the statusline bridge and the usage API consent.
  Section ids (`dirs`, `statusline`, `api`) are the targets of the panel's links.
-->
<script lang="ts">
import Button from "../../lib/components/Button.svelte";
import Card from "../../lib/components/Card.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import { commands } from "../../lib/ipc/bindings";
import { clock } from "../../lib/time/clock.svelte";
import ApiConsent from "./ApiConsent.svelte";
import StatuslineDialog from "./StatuslineDialog.svelte";
import { loadSources, store } from "./state.svelte";
import { pickErrorKey } from "./view";

let { onerror }: { onerror: (key: string | null) => void } = $props();

let dialog: StatuslineDialog;
let notice = $state<string | null>(null);
let busy = $state(false);

const sources = $derived(store.sources);
const statusline = $derived(sources?.statusline ?? null);
const currentCommand = $derived(
  statusline?.state === "not_installed" ? statusline.current_command : null,
);

async function pick() {
  busy = true;
  const r = await commands.pickClaudeDir();
  busy = false;
  if (r.status === "error") {
    onerror(pickErrorKey(r.error));
    return;
  }
  onerror(null);
  if (r.data) await loadSources();
}

async function resetDir() {
  busy = true;
  const r = await commands.resetClaudeDir();
  busy = false;
  onerror(r.status === "ok" ? null : "settings-save-failed");
  await loadSources();
}

function openDialog(mode: "install" | "uninstall") {
  notice = null;
  void dialog.open(mode);
}

async function dialogDone(result: { backup: string | null } | null) {
  if (!result) return;
  notice = result.backup ? t("statusline-done", { path: result.backup }) : null;
  await loadSources();
}
</script>

<section id="dirs" tabindex="-1">
  <Card title={t("sources-dirs")}>
    <p class="hint">{t("sources-dirs-hint")}</p>
    {#if sources}
      {#if sources.chosen_missing}
        <p class="note warn">{t("sources-dir-missing")}</p>
        <p class="path"><code>{sources.chosen_missing}</code></p>
      {/if}
      {#if sources.dirs.length === 0}
        <p class="note">{t("sources-dir-none")}</p>
      {:else}
        <ul class="dirs">
          {#each sources.dirs as dir (dir.path)}
            <li>
              <div class="dir-head">
                <code>{dir.path}</code>
                {#if dir.primary}<span class="tag">{t("sources-dir-primary")}</span>{/if}
                {#if dir.chosen}<span class="tag">{t("sources-dir-chosen")}</span>{/if}
                {#if sources.env_dir === dir.path}<span class="tag">{t("sources-dir-env")}</span>{/if}
              </div>
              {#if dir.stats}
                <p class="meta">
                  {t("sources-dir-stats", {
                    sessions: dir.stats.sessions,
                    projects: dir.stats.projects,
                    files: dir.stats.files,
                  })}
                </p>
                {#if dir.stats.last_activity !== null || dir.stats.last_cc_version}
                  <p class="meta">
                    {#if dir.stats.last_activity !== null}
                      {t("sources-dir-last", { ms: Math.max(0, clock.now - dir.stats.last_activity) })}
                    {/if}
                    {#if dir.stats.last_activity !== null && dir.stats.last_cc_version}·{/if}
                    {#if dir.stats.last_cc_version}
                      {t("sources-dir-version", { version: dir.stats.last_cc_version })}
                    {/if}
                  </p>
                {/if}
              {:else}
                <p class="meta">{t("sources-dir-pending")}</p>
              {/if}
            </li>
          {/each}
        </ul>
      {/if}
      <p class="note">{t("sources-change-note")}</p>
      <div class="actions">
        <Button disabled={busy} onclick={pick}>{t("sources-change")}</Button>
        {#if sources.dirs.some((d) => d.chosen) || sources.chosen_missing}
          <Button disabled={busy} onclick={resetDir}>{t("sources-reset-dir")}</Button>
        {/if}
      </div>
    {/if}
  </Card>
</section>

<section id="statusline" tabindex="-1">
  <Card title={t("statusline-title")}>
    {#snippet corner()}<span class="tag">{t("statusline-recommended")}</span>{/snippet}
    <p class="hint">{t("statusline-hint")}</p>
    {#if sources && statusline}
      {#if statusline.state === "no_directory"}
        <p class="note">{t("statusline-no-directory")}</p>
      {:else if statusline.state === "not_installed"}
        <p class="state">{t("statusline-not-installed")}</p>
        {#if statusline.current_command}
          <p class="meta">{t("statusline-current")}</p>
          <p class="path"><code>{statusline.current_command}</code></p>
        {/if}
      {:else if statusline.state === "installed"}
        {#if statusline.chained_command}
          <p class="state">{t("statusline-installed-chained")}</p>
          <p class="path"><code>{statusline.chained_command}</code></p>
        {:else}
          <p class="state">{t("statusline-installed")}</p>
        {/if}
      {:else}
        <p class="state">{t("statusline-installed-elsewhere")}</p>
      {/if}

      {#if statusline.state === "installed" || statusline.state === "installed_elsewhere"}
        <p class="meta">
          {#if sources.statusline_last_seen !== null}
            {t("statusline-last-seen", { ms: Math.max(0, clock.now - sources.statusline_last_seen) })}
          {:else}
            {t("statusline-never-seen")}
          {/if}
        </p>
      {/if}

      {#if sources.cli_missing && statusline.state === "not_installed"}
        <p class="note warn">{t("statusline-cli-missing")}</p>
      {/if}
      {#if notice}<p class="note" role="status">{notice}</p>{/if}

      <div class="actions">
        {#if statusline.state === "not_installed"}
          <Button variant="primary" disabled={sources.cli_missing} onclick={() => openDialog("install")}>
            {t("statusline-install")}
          </Button>
        {:else if statusline.state === "installed"}
          <Button onclick={() => openDialog("uninstall")}>{t("statusline-uninstall")}</Button>
        {/if}
      </div>
    {/if}
  </Card>
</section>

<section id="api" tabindex="-1">
  <Card title={t("api-title")}>
    <p class="hint">{t("api-hint")}</p>
    <ApiConsent {onerror} />
  </Card>
</section>

<StatuslineDialog bind:this={dialog} {currentCommand} ondone={dialogDone} />

<style>
  section {
    outline: none;
    scroll-margin-top: var(--space-3);
  }
  .hint {
    color: var(--text-2);
    margin-bottom: var(--space-2);
  }
  .meta,
  .note {
    font: var(--type-caption);
    color: var(--text-2);
    margin-top: var(--space-1);
  }
  .state {
    margin-top: var(--space-1);
  }
  .warn {
    color: var(--text-1);
    border-left: 2px solid var(--signal-warn);
    padding-left: var(--space-2);
  }
  code {
    font: 400 12px / 16px var(--font-mono);
    user-select: text;
    word-break: break-all;
  }
  .path {
    margin-top: var(--space-1);
    color: var(--text-2);
  }
  .dirs li {
    padding: var(--space-2) 0;
    border-top: 1px solid var(--stroke);
  }
  .dirs li:last-child {
    border-bottom: 1px solid var(--stroke);
  }
  .dir-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }
  .tag {
    font: var(--type-micro);
    padding: 1px var(--space-2);
    border-radius: var(--radius-pill);
    border: 1px solid var(--stroke);
    color: var(--text-2);
  }
  .actions {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-3);
  }
</style>

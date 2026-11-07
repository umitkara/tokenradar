<!--
  TokenRadar's Claude Code mod (ADR-047): listed in `env.CLAUDE_CODE_PLUGIN_DIRS` of the primary
  account's `settings.json`, through the same diff dialog as the statusline bridge. Used by the
  settings' data sources and by onboarding, where nothing is installed unless the user asks.
-->
<script lang="ts">
import Button from "../../lib/components/Button.svelte";
import Card from "../../lib/components/Card.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import StatuslineDialog from "./StatuslineDialog.svelte";
import { loadSources, store } from "./state.svelte";

let dialog: StatuslineDialog;
let notice = $state<string | null>(null);

const mod = $derived(store.sources?.claude_mod ?? null);

function openDialog(mode: "install" | "uninstall") {
  notice = null;
  void dialog.open(mode);
}

async function dialogDone(result: { backup: string | null } | null) {
  if (!result) return;
  const done = t(mod?.state === "not_installed" ? "mod-done-install" : "mod-done-uninstall");
  notice = result.backup ? `${done} ${t("mod-backup", { path: result.backup })}` : done;
  await loadSources();
}
</script>

<Card title={t("mod-title")}>
  {#snippet corner()}<span class="tag">{t("mod-experimental")}</span>{/snippet}
  <p class="hint">{t("mod-hint")}</p>
  {#if mod}
    {#if mod.state === "no_directory"}
      <p class="note">{t("statusline-no-directory")}</p>
    {:else if mod.state === "missing"}
      <p class="note warn">{t("mod-missing")}</p>
    {:else if mod.state === "not_installed"}
      <p class="state">{t("statusline-not-installed")}</p>
    {:else}
      <p class="state">{t("mod-installed")}</p>
      <p class="path"><code>{mod.path}</code></p>
    {/if}
    {#if notice}<p class="note" role="status">{notice}</p>{/if}
    <div class="actions">
      {#if mod.state === "not_installed"}
        <Button onclick={() => openDialog("install")}>{t("statusline-install")}</Button>
      {:else if mod.state === "installed"}
        <Button onclick={() => openDialog("uninstall")}>{t("statusline-uninstall")}</Button>
      {/if}
    </div>
  {/if}
</Card>

<StatuslineDialog bind:this={dialog} target="mod" currentCommand={null} ondone={dialogDone} />

<style>
  .hint {
    color: var(--text-2);
    margin-bottom: var(--space-2);
  }
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
  .path code {
    font: var(--type-caption);
    font-family: var(--font-mono);
    color: var(--text-2);
    user-select: text;
    word-break: break-all;
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
    margin-top: var(--space-2);
  }
  .actions:empty {
    display: none;
  }
</style>

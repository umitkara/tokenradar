<!--
  Step 3: where limit percentages come from. The statusline bridge (recommended) installs with
  the same diff dialog as the settings; the usage API shows the same consent text. Both can be
  skipped, and the line at the bottom says plainly what that means.
-->
<script lang="ts">
import Button from "../../lib/components/Button.svelte";
import Card from "../../lib/components/Card.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import { clock } from "../../lib/time/clock.svelte";
import ApiConsent from "../settings/ApiConsent.svelte";
import StatuslineDialog from "../settings/StatuslineDialog.svelte";
import { loadSources, store } from "../settings/state.svelte";
import { limitsOutcome, OUTCOME_KEYS } from "./view";

let { onerror }: { onerror: (key: string | null) => void } = $props();

let dialog: StatuslineDialog;
let notice = $state<string | null>(null);

const sources = $derived(store.sources);
const statusline = $derived(sources?.statusline ?? null);
const currentCommand = $derived(
  statusline?.state === "not_installed" ? statusline.current_command : null,
);
const outcome = $derived(limitsOutcome(sources));

async function dialogDone(result: { backup: string | null } | null) {
  if (!result) return;
  notice = result.backup ? t("statusline-done", { path: result.backup }) : null;
  await loadSources();
}
</script>

<p class="intro">{t("onb-limits-intro")}</p>

<Card title={t("statusline-title")}>
  {#snippet corner()}<span class="tag">{t("statusline-recommended")}</span>{/snippet}
  <p class="hint">{t("statusline-hint")}</p>
  {#if sources && statusline}
    {#if statusline.state === "no_directory"}
      <p class="note">{t("statusline-no-directory")}</p>
    {:else if statusline.state === "not_installed"}
      {#if sources.cli_missing}<p class="note warn">{t("statusline-cli-missing")}</p>{/if}
      <div class="actions">
        <Button
          variant="primary"
          disabled={sources.cli_missing}
          onclick={() => dialog.open("install")}
        >
          {t("statusline-install")}
        </Button>
      </div>
    {:else}
      <p class="state">
        {t(statusline.state === "installed" ? "statusline-installed" : "statusline-installed-elsewhere")}
      </p>
      <p class="meta">
        {#if sources.statusline_last_seen !== null}
          {t("statusline-last-seen", { ms: Math.max(0, clock.now - sources.statusline_last_seen) })}
        {:else}
          {t("statusline-never-seen")}
        {/if}
      </p>
    {/if}
    {#if notice}<p class="note" role="status">{notice}</p>{/if}
  {/if}
</Card>

<Card title={t("api-title")}>
  {#snippet corner()}<span class="tag">{t("onb-limits-api-optional")}</span>{/snippet}
  <p class="hint">{t("api-hint")}</p>
  <details open={sources?.api.granted ?? false}>
    <summary>{t("onb-limits-api-show")}</summary>
    <div class="consent"><ApiConsent {onerror} /></div>
  </details>
</Card>

{#if outcome}
  <p class="outcome" class:skipped={outcome === "none"} role="status">{t(OUTCOME_KEYS[outcome])}</p>
{/if}

<StatuslineDialog bind:this={dialog} {currentCommand} ondone={dialogDone} />

<style>
  .intro,
  .hint {
    color: var(--text-2);
  }
  .hint {
    margin-bottom: var(--space-2);
  }
  .meta,
  .note {
    font: var(--type-caption);
    color: var(--text-2);
    margin-top: var(--space-1);
  }
  .warn {
    color: var(--text-1);
    border-left: 2px solid var(--signal-warn);
    padding-left: var(--space-2);
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
  summary {
    cursor: pointer;
    color: var(--accent);
    width: fit-content;
    border-radius: var(--radius-control);
  }
  .consent {
    margin-top: var(--space-2);
  }
  .outcome {
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-control);
    background: var(--surface-1);
  }
  .skipped {
    border-left: 2px solid var(--signal-est);
  }
</style>

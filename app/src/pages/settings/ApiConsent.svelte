<!--
  The usage API consent: the full explanation, the poller's state and the allow / turn off
  button. Settings and onboarding both show this one component, so the text a consent is
  versioned against (`API_CONSENT_VERSION`) is the same everywhere.
-->
<script lang="ts">
import Button from "../../lib/components/Button.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import { commands } from "../../lib/ipc/bindings";
import { clock } from "../../lib/time/clock.svelte";
import { store } from "./state.svelte";
import { apiStatus, consentAge } from "./view";

let { onerror }: { onerror: (key: string | null) => void } = $props();

let busy = $state(false);

const api = $derived(store.sources?.api ?? null);
const apiLines = $derived(api ? apiStatus(api, clock.now) : []);
const allowedAgo = $derived(api ? consentAge(api, clock.now) : null);

async function setConsent(granted: boolean) {
  busy = true;
  const r = await commands.setApiConsent(granted);
  busy = false;
  if (r.status === "ok") {
    store.sources = r.data;
    onerror(null);
  } else {
    onerror("settings-save-failed");
  }
}
</script>

<dl class="consent">
  <dt>{t("api-what-label")}</dt>
  <dd>{t("api-what")}</dd>
  <dt>{t("api-reads-label")}</dt>
  <dd>{t("api-reads")}</dd>
  <dt>{t("api-never-label")}</dt>
  <dd>{t("api-never")}</dd>
  <dt>{t("api-where-label")}</dt>
  <dd>{t("api-where")}</dd>
</dl>
<p class="meta">{t("api-revoke")}</p>
{#if api}
  {#if api.outdated}
    <p class="note warn">{t("api-outdated")}</p>
  {/if}
  {#if api.granted}
    {#if allowedAgo !== null}<p class="state">{t("api-allowed", { ms: allowedAgo })}</p>{/if}
    {#each apiLines as line (line.key)}
      <p class="meta">{t(line.key, line.args)}</p>
    {/each}
  {/if}
  <div class="actions">
    {#if api.granted}
      <Button disabled={busy} onclick={() => setConsent(false)}>{t("api-turn-off")}</Button>
    {:else}
      <Button disabled={busy} onclick={() => setConsent(true)}>{t("api-allow")}</Button>
    {/if}
  </div>
{/if}

<style>
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
  .consent {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--space-1) var(--space-3);
    margin: 0 0 var(--space-2);
  }
  .consent dt {
    font-weight: 600;
  }
  .consent dd {
    margin: 0;
    color: var(--text-1);
  }
  .actions {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-3);
  }
</style>

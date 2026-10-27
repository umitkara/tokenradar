<!--
  About: version, updates (the daily check, "Check now", "Install and restart"), the licenses
  and the project's pages. Links open only the fixed GitHub pages in `privacy.rs` (ADR-013).
-->
<script lang="ts">
import Button from "../../lib/components/Button.svelte";
import Card from "../../lib/components/Card.svelte";
import Toggle from "../../lib/components/Toggle.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import { commands, type Link } from "../../lib/ipc/bindings";
import { clock } from "../../lib/time/clock.svelte";
import Field from "./Field.svelte";
import { savePrefs, store } from "./state.svelte";
import { updateUi } from "./view";

let { onerror }: { onerror: (key: string | null) => void } = $props();

const update = $derived(store.update ? updateUi(store.update, clock.now) : null);
const notes = $derived(store.update?.available?.notes ?? null);

async function open(link: Link) {
  const r = await commands.openLink(link);
  onerror(r.status === "ok" ? null : "about-link-failed");
}

async function setUpdateCheck(update_check: boolean) {
  const prefs = store.prefs;
  if (!prefs) return;
  const data = { ...prefs.data, update_check };
  onerror((await savePrefs(() => commands.setDataPrefs(data))) ? null : "settings-save-failed");
}

async function checkNow() {
  store.update = await commands.checkForUpdate();
}

async function install() {
  // Progress and failure arrive as `updateChanged`; on success the app exits.
  await commands.installUpdate();
}
</script>

<Card>
  <h2>{t("app-name")}</h2>
  {#if store.about}<p class="version">{t("about-version", { version: store.about.version })}</p>{/if}
  <p class="hint">{t("about-tagline")}</p>
  <div class="actions">
    <Button onclick={() => open("repo")}>{t("about-repo")}</Button>
    <Button onclick={() => open("issues")}>{t("about-issues")}</Button>
  </div>
</Card>

<section id="updates" tabindex="-1">
  <Card title={t("about-updates")}>
    {#if store.prefs}
      <Field label={t("about-update-check")} hint={t("about-update-check-hint")}>
        <Toggle
          checked={store.prefs.data.update_check ?? true}
          label={t("about-update-check")}
          labelHidden
          onchange={setUpdateCheck}
        />
      </Field>
    {/if}
    {#if update}
      <div aria-live="polite">
        {#each update.lines as line (line.key)}
          <p class="state">{t(line.key, line.args)}</p>
        {/each}
      </div>
      {#if notes}
        <details>
          <summary>{t("about-update-notes")}</summary>
          <p class="notes">{notes}</p>
        </details>
      {/if}
    {/if}
    <div class="actions">
      {#if update?.install}
        <Button variant="primary" disabled={update.busy} onclick={install}>
          {t("about-update-install")}
        </Button>
      {/if}
      <Button disabled={update?.busy ?? true} onclick={checkNow}>{t("about-update-now")}</Button>
      <Button onclick={() => open("releases")}>{t("about-releases")}</Button>
    </div>
  </Card>
</section>

<Card title={t("about-license")}>
  <p>{t("about-license-text")}</p>
  {#if store.about?.third_party_notices}
    <p class="hint">{t("about-third-party")}</p>
  {:else}
    <p class="hint">{t("about-third-party-missing")}</p>
  {/if}
  <div class="actions">
    <Button onclick={() => open("license")}>{t("about-license")}</Button>
    {#if store.about?.third_party_notices}
      <Button onclick={() => open("third_party_notices")}>{t("about-third-party-open")}</Button>
    {/if}
  </div>
</Card>

<style>
  h2 {
    font: var(--type-title);
  }
  section {
    outline: none;
  }
  .version {
    font-family: var(--font-mono);
    color: var(--text-2);
    user-select: text;
  }
  .hint,
  .state {
    font: var(--type-caption);
    color: var(--text-2);
    margin-top: var(--space-1);
  }
  details {
    margin-top: var(--space-2);
    font: var(--type-caption);
  }
  summary {
    cursor: pointer;
    color: var(--text-2);
  }
  .notes {
    white-space: pre-wrap;
    user-select: text;
    max-height: 160px;
    overflow-y: auto;
    margin-top: var(--space-1);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-top: var(--space-3);
  }
</style>

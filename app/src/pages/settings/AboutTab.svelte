<!--
  About: version, the update-check switch (the updater arrives in Step 12), the license and the
  project's pages. Links open only the fixed GitHub pages in `privacy.rs` (ADR-013).
-->
<script lang="ts">
import Button from "../../lib/components/Button.svelte";
import Card from "../../lib/components/Card.svelte";
import Toggle from "../../lib/components/Toggle.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import { commands, type Link } from "../../lib/ipc/bindings";
import Field from "./Field.svelte";
import { savePrefs, store } from "./state.svelte";

let { onerror }: { onerror: (key: string | null) => void } = $props();

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

<Card title={t("about-updates")}>
  {#if store.prefs}
    <Field label={t("about-update-check")} hint={t("about-update-later")}>
      <Toggle
        checked={store.prefs.data.update_check ?? true}
        label={t("about-update-check")}
        labelHidden
        onchange={setUpdateCheck}
      />
    </Field>
  {/if}
  <div class="actions">
    <Button onclick={() => open("releases")}>{t("about-releases")}</Button>
  </div>
</Card>

<Card title={t("about-license")}>
  <p>{t("about-license-text")}</p>
  <p class="hint">{t("about-third-party")}</p>
  <div class="actions">
    <Button onclick={() => open("license")}>{t("about-license")}</Button>
  </div>
</Card>

<style>
  h2 {
    font: var(--type-title);
  }
  .version {
    font-family: var(--font-mono);
    color: var(--text-2);
    user-select: text;
  }
  .hint {
    font: var(--type-caption);
    color: var(--text-2);
    margin-top: var(--space-1);
  }
  .actions {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-3);
  }
</style>

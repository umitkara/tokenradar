<!-- Step 1: what TokenRadar does, and the language (the detected one is preselected). -->
<script lang="ts">
import Select from "../../lib/components/Select.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import { commands, type UiLocale } from "../../lib/ipc/bindings";
import { inTauri } from "../../lib/ipc/runtime";
import { settings } from "../../lib/settings.svelte";

let { onerror }: { onerror: (key: string | null) => void } = $props();

// Language names are written in their own language, so both read the same in EN and TR.
const locales = $derived<{ value: UiLocale; label: string }[]>([
  { value: "en", label: t("language-en") },
  { value: "tr", label: t("language-tr") },
]);

async function setLocale(value: UiLocale) {
  if (!inTauri()) return;
  // `settings-changed` re-renders every open window in the new language.
  const r = await commands.setLocale(value);
  onerror(r.status === "ok" ? null : "settings-save-failed");
}
</script>

<p class="pitch">{t("onb-welcome-pitch")}</p>
<p class="local">{t("onb-welcome-local")}</p>
<div class="field">
  <label for="language">{t("general-language")}</label>
  <Select id="language" value={settings.view.locale} options={locales} onchange={setLocale} />
</div>

<style>
  .pitch {
    font: var(--type-title);
    font-weight: 400;
    max-width: 52ch;
  }
  .local {
    color: var(--text-2);
  }
  .field {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-top: var(--space-3);
  }
</style>

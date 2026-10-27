<!-- General: language, theme, starting with Windows and how long a closed panel stays ready. -->
<script lang="ts">
import Card from "../../lib/components/Card.svelte";
import Select from "../../lib/components/Select.svelte";
import Toggle from "../../lib/components/Toggle.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import { commands, type LocalePref, type ThemePref } from "../../lib/ipc/bindings";
import { settings } from "../../lib/settings.svelte";
import Field from "./Field.svelte";
import { savePrefs, store } from "./state.svelte";
import { optionsWith, WARM_TTL_PRESETS } from "./view";

let { onerror }: { onerror: (key: string | null) => void } = $props();

const locales = $derived<{ value: LocalePref; label: string }[]>([
  { value: "system", label: t("general-language-system") },
  { value: "en", label: t("language-en") },
  { value: "tr", label: t("language-tr") },
]);
const themes = $derived<{ value: ThemePref; label: string }[]>([
  { value: "system", label: t("general-theme-system") },
  { value: "light", label: t("general-theme-light") },
  { value: "dark", label: t("general-theme-dark") },
]);
const warm = $derived(
  optionsWith(WARM_TTL_PRESETS, store.prefs?.panel_warm_ttl_s ?? 60).map((s) => ({
    value: s,
    label: s === 0 ? t("general-warm-off") : t("general-warm-option", { ms: s * 1000 }),
  })),
);

// The language and theme answer with `SettingsView`, which `settings-changed` also delivers to
// every window; lib/settings applies it.
async function setLocale(value: LocalePref) {
  const r = await commands.setLocale(value);
  onerror(r.status === "ok" ? null : "settings-save-failed");
}

async function setTheme(value: ThemePref) {
  const r = await commands.setTheme(value);
  onerror(r.status === "ok" ? null : "settings-save-failed");
}

async function setAutostart(enabled: boolean) {
  const r = await commands.setAutostart(enabled);
  if (r.status === "ok") {
    store.autostart = r.data;
    onerror(null);
  } else {
    onerror("settings-save-failed");
  }
}

async function setWarm(seconds: number) {
  onerror(
    (await savePrefs(() => commands.setPanelWarmTtl(seconds))) ? null : "settings-save-failed",
  );
}
</script>

<Card>
  <Field label={t("general-language")} id="language">
    <Select id="language" value={settings.view.locale_pref} options={locales} onchange={setLocale} />
  </Field>
  <Field label={t("general-theme")} id="theme">
    <Select id="theme" value={settings.view.theme_pref} options={themes} onchange={setTheme} />
  </Field>
</Card>

<Card>
  <Field
    label={t("general-autostart")}
    hint={store.autostart === null ? t("general-autostart-unknown") : t("general-autostart-hint")}
  >
    <Toggle
      checked={store.autostart ?? false}
      label={t("general-autostart")}
      labelHidden
      disabled={store.autostart === null}
      onchange={setAutostart}
    />
  </Field>
  {#if store.prefs}
    <Field label={t("general-warm")} id="warm" hint={t("general-warm-hint")}>
      <Select id="warm" value={store.prefs.panel_warm_ttl_s} options={warm} onchange={setWarm} />
    </Field>
  {/if}
</Card>

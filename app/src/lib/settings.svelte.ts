// Applies the user's settings to the page: `<html lang>`, `data-theme`, `data-os` and the i18n
// language. The first values come from the boot script Rust injects before the page loads
// (src-tauri/src/chrome.rs), so the first render is already right; later changes arrive as
// `settings-changed` events.

import { type Catalog, initI18n, setLocale } from "./i18n/i18n.svelte";
import { commands, events, type SettingsView, type ThemePref } from "./ipc/bindings";
import { inTauri } from "./ipc/runtime";

declare global {
  interface Window {
    __TOKENRADAR_BOOT__?: SettingsView;
  }
}

const darkQuery = () =>
  typeof matchMedia === "function" ? matchMedia("(prefers-color-scheme: dark)") : null;

/** A browser without the app (Vite dev server, gallery): follow the browser. */
function fallback(): SettingsView {
  const tr = typeof navigator !== "undefined" && navigator.language.toLowerCase().startsWith("tr");
  return {
    locale_pref: "system",
    theme_pref: "system",
    locale: tr ? "tr" : "en",
    os: "win10",
    hour12: false,
  };
}

export const settings = $state<{ view: SettingsView }>({ view: fallback() });

/** Writes `data-theme` for `pref`; `system` follows `prefers-color-scheme`. */
export function applyTheme(pref: ThemePref): void {
  const dark = pref === "system" ? (darkQuery()?.matches ?? true) : pref === "dark";
  document.documentElement.dataset.theme = dark ? "dark" : "light";
}

function apply(view: SettingsView): void {
  settings.view = view;
  document.documentElement.dataset.os = view.os;
  applyTheme(view.theme_pref);
  setLocale(view.locale, view.hour12);
}

/** Call first in every page's `main.ts`, before mounting. */
export function boot(catalog: Catalog): void {
  const view = window.__TOKENRADAR_BOOT__ ?? fallback();
  initI18n(catalog, view.locale, view.hour12);
  apply(view);
  darkQuery()?.addEventListener("change", () => {
    if (settings.view.theme_pref === "system") applyTheme("system");
  });
  if (inTauri()) {
    void events.settingsChanged.listen((e) => apply(e.payload));
    // The boot values may predate config.toml being read (src-tauri/src/prefs.rs).
    void commands.getSettings().then(apply);
  }
}

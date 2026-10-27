// The page's language: one rune-backed state, `t()` for messages and `<html lang>`.
// Each page passes only the `.ftl` domains it uses (see lib/i18n/domains.ts).

import type { FluentBundle } from "@fluent/bundle";
import { type Args, createBundle, format } from "./fluent";
import type { Locale } from "./format";

export type Catalog = Record<Locale, readonly string[]>;

const state = $state({ locale: "en" as Locale, hour12: false });
let catalog: Catalog = { en: [], tr: [] };
const bundles = new Map<Locale, FluentBundle>();

/** Registers the page's messages. Call once, before the first render. */
export function initI18n(messages: Catalog, locale: Locale, hour12 = false): void {
  catalog = messages;
  bundles.clear();
  setLocale(locale, hour12);
}

/** Switches the language without a reload; every `t()` in a template re-renders. */
export function setLocale(locale: Locale, hour12 = state.hour12): void {
  state.locale = locale;
  state.hour12 = locale === "en" && hour12;
  if (typeof document !== "undefined") document.documentElement.lang = locale;
}

export function locale(): Locale {
  return state.locale;
}

/** English follows the OS region's 12/24-hour choice; Turkish is always 24-hour. */
export function hour12(): boolean {
  return state.hour12;
}

function bundle(l: Locale): FluentBundle {
  let b = bundles.get(l);
  if (!b) {
    b = createBundle(l, catalog[l]);
    bundles.set(l, b);
  }
  return b;
}

/** A localized message. Reactive: reads the current language. */
export function t(key: string, args?: Args): string {
  return format(bundle(state.locale), key, args);
}

/** Locale-aware upper case: Turkish `i` → `İ`. Never use `toUpperCase()` on UI text. */
export function upper(text: string): string {
  return text.toLocaleUpperCase(state.locale);
}

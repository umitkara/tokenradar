// Fluent bundles with TokenRadar's custom functions (architecture §12), shared by every page.
// Behaves like crates/tokenradar-i18n/src/lib.rs: no isolation marks, and a missing message
// shows its id so the gap is visible.

import { FluentBundle, FluentNumber, FluentResource, type FluentValue } from "@fluent/bundle";
import { dur, type Locale, num, parseDurStyle, pct, shortestDecimals } from "./format";
import { parseCase, trSuffix } from "./suffix";

export type Args = Record<string, string | number>;

function number(v: FluentValue | undefined): number {
  if (v instanceof FluentNumber) return v.value;
  if (typeof v === "string") return Number(v) || 0;
  return 0;
}

/** A bundle for `locale` holding `sources` (`.ftl` text). */
export function createBundle(locale: Locale, sources: readonly string[]): FluentBundle {
  const bundle = new FluentBundle(locale, {
    useIsolating: false,
    functions: {
      NUM: ([n, decimals]) =>
        num(locale, number(n), Math.min(6, Math.max(0, Math.trunc(number(decimals))))),
      PCT: ([n]) => pct(locale, number(n)),
      DUR: ([ms, style]) => dur(locale, Math.trunc(number(ms)), parseDurStyle(style)),
      TR_SUFFIX: ([value, kase]) => {
        const text =
          value instanceof FluentNumber
            ? num(locale, value.value, shortestDecimals(value.value))
            : typeof value === "string"
              ? value
              : "";
        const c = parseCase(kase);
        return locale === "tr" && c ? trSuffix(text, c) : text;
      },
    },
  });
  for (const source of sources) {
    const errors = bundle.addResource(new FluentResource(source));
    if (errors.length > 0 && import.meta.env.DEV) console.error(errors);
  }
  return bundle;
}

/** The message `key` with `args`; the key itself when the message is missing. */
export function format(bundle: FluentBundle, key: string, args?: Args): string {
  const message = bundle.getMessage(key);
  if (!message?.value) return key;
  const errors: Error[] = [];
  const out = bundle.formatPattern(message.value, args, errors);
  if (errors.length > 0 && import.meta.env.DEV) console.error(key, errors);
  return out;
}

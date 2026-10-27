// The TypeScript half of the `NUM`, `PCT` and `DUR` Fluent functions and the clock formatter.
// A line-by-line port of crates/tokenradar-i18n/src/format.rs, not `Intl`: tests run on Node's
// ICU and the app on WebView2's, and only a port gives both the same strings (ADR-031).
// testdata/i18n-vectors.json pins the output for both languages.

export type Locale = "en" | "tr";
export type DurStyle = "short" | "compact" | "long";

/** `n` with `decimals` fraction digits and locale grouping: TR `1.234,5`, EN `1,234.5`. */
export function num(locale: Locale, n: number, decimals: number): string {
  const [group, point] = locale === "tr" ? [".", ","] : [",", "."];
  const fixed = toFixedHalfEven(Math.abs(n), decimals);
  const [int = "", frac = ""] = fixed.split(".");
  let out = "";
  // `-0` after rounding is printed as `0`.
  if (n < 0 && /[1-9]/.test(fixed)) out += "-";
  for (let i = 0; i < int.length; i++) {
    if (i > 0 && (int.length - i) % 3 === 0) out += group;
    out += int[i];
  }
  if (frac) out += point + frac;
  return out;
}

/**
 * `x.toFixed(digits)`, except that exact ties round to even like Rust's `{:.N}` (`0.125` →
 * `0.12`); `toFixed` rounds them up. Values that only look like ties (`1.005`) are not exact in
 * binary and agree already.
 */
function toFixedHalfEven(x: number, digits: number): string {
  const rounded = x.toFixed(digits);
  const exact = x.toFixed(100);
  const dot = exact.indexOf(".");
  const tail = exact.slice(dot + 1 + digits);
  if (tail[0] !== "5" || /[1-9]/.test(tail.slice(1))) return rounded;
  const truncated = exact.slice(0, digits > 0 ? dot + 1 + digits : dot);
  const last = Number(truncated[truncated.length - 1]);
  return last % 2 === 0 ? truncated : rounded;
}

/** A percentage with at most one decimal, trailing `,0` dropped: TR `%72`, `%23,5`; EN `72%`. */
export function pct(locale: Locale, n: number): string {
  const rounded = roundHalfAway(n * 10) / 10;
  const decimals = rounded % 1 !== 0 ? 1 : 0;
  const digits = num(locale, rounded, decimals);
  return locale === "tr" ? `%${digits}` : `${digits}%`;
}

/** Rust's `f64::round`: halves away from zero (`Math.round` sends -2.5 to -2). */
function roundHalfAway(x: number): number {
  return Math.sign(x) * Math.round(Math.abs(x));
}

const SEC = 1000;
const MIN = 60 * SEC;
const HOUR = 60 * MIN;
const DAY = 24 * HOUR;

type Unit = "day" | "hour" | "min" | "sec";

const UNITS: Record<Locale, Record<Unit, string>> = {
  en: { day: "d", hour: "h", min: "m", sec: "s" },
  tr: { day: "g", hour: "sa", min: "dk", sec: "sn" },
};

/**
 * A duration in its two largest units, truncated: `59 sn`, `1 dk`, `1 sa 48 dk`, `2 g 3 sa`
 * (EN `59s`, `1m`, `1h 48m`, `2d 3h`). Negative durations print as zero.
 */
export function dur(locale: Locale, ms: number, style: DurStyle = "short"): string {
  const t = Math.max(0, Math.trunc(ms));
  let big: [number, Unit];
  let small: [number, Unit] | null = null;
  if (t >= DAY) {
    big = [Math.floor(t / DAY), "day"];
    small = [Math.floor((t % DAY) / HOUR), "hour"];
  } else if (t >= HOUR) {
    big = [Math.floor(t / HOUR), "hour"];
    small = [Math.floor((t % HOUR) / MIN), "min"];
  } else if (t >= MIN) {
    big = [Math.floor(t / MIN), "min"];
  } else {
    big = [Math.floor(t / SEC), "sec"];
  }
  let out = part(locale, big[0], big[1], style);
  if (small && small[0] > 0) {
    if (style !== "compact") out += " ";
    out += part(locale, small[0], small[1], style);
  }
  return out;
}

const WORDS: Record<Locale, Record<Unit, string>> = {
  en: { day: "day", hour: "hour", min: "minute", sec: "second" },
  tr: { day: "gün", hour: "saat", min: "dakika", sec: "saniye" },
};

function part(locale: Locale, n: number, unit: Unit, style: DurStyle): string {
  if (style === "long") {
    // Words, for screen readers. Turkish nouns stay singular after a number.
    const plural = locale === "en" && n !== 1 ? "s" : "";
    return `${n} ${WORDS[locale][unit]}${plural}`;
  }
  // Turkish separates number and unit in the long form; English never does.
  const sep = locale === "tr" && style === "short" ? " " : "";
  return `${n}${sep}${UNITS[locale][unit]}`;
}

/** `"compact"`, `"long"` or anything else (`"short"`), like the Rust `DurStyle::parse`. */
export function parseDurStyle(s: unknown): DurStyle {
  return s === "compact" || s === "long" ? s : "short";
}

const WEEKDAYS: Record<Locale, string[]> = {
  en: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"],
  tr: ["Paz", "Pzt", "Sal", "Çar", "Per", "Cum", "Cmt"],
};

/** A wall-clock time: TR `09:05`; EN `09:05` or `9:05 AM`. Turkish is always 24-hour. */
export function clock(locale: Locale, hour: number, minute: number, hour12: boolean): string {
  const mm = String(minute).padStart(2, "0");
  if (locale === "en" && hour12) {
    const h = hour % 12 === 0 ? 12 : hour % 12;
    return `${h}:${mm} ${hour < 12 ? "AM" : "PM"}`;
  }
  return `${String(hour).padStart(2, "0")}:${mm}`;
}

/** Weekday (`0` = Sunday, as `Date#getDay`) and time: TR `Çar 10:59`, EN `Wed 10:59 AM`. */
export function weekdayClock(
  locale: Locale,
  weekday: number,
  hour: number,
  minute: number,
  hour12: boolean,
): string {
  return `${WEEKDAYS[locale][((weekday % 7) + 7) % 7]} ${clock(locale, hour, minute, hour12)}`;
}

/** Fraction digits needed to print `n` without trailing zeros (at most 6). */
export function shortestDecimals(n: number): number {
  for (let d = 0; d < 6; d++) {
    const scaled = n * 10 ** d;
    if (Math.abs(Math.round(scaled) - scaled) < 1e-9) return d;
  }
  return 6;
}

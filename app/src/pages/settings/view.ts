// The settings window's pure logic: which tab a panel link opens, how sizes and API states are
// worded, and the rules for threshold chips. Components only render what these return.

import type { GlyphRun } from "../../lib/components/glyphs";
import type { Args } from "../../lib/i18n/fluent";
import type {
  AnsiColor,
  ApiConsentView,
  BarStyle,
  ColorMode,
  Format,
  Group,
  IconSet,
  Item,
  NerdFontView,
  PickError,
  ResetFormat,
  SampleState,
  SegmentKind,
  SettingsSection,
  Span,
  StatuslineError,
  StatuslineKind,
  StatuslinePrefs,
  StatuslinePreview,
  Theme,
  UpdateView,
} from "../../lib/ipc/bindings";

export type Tab = "general" | "sources" | "statusline" | "notifications" | "privacy" | "about";

export const TABS: readonly Tab[] = [
  "general",
  "sources",
  "statusline",
  "notifications",
  "privacy",
  "about",
];

/** A localized message: its key and arguments. */
export interface Message {
  key: string;
  args?: Args;
}

/** Where a panel link lands: a tab and, inside it, the element with id `anchor`. */
export interface Target {
  tab: Tab;
  anchor: string | null;
}

const TARGETS: Record<SettingsSection, Target> = {
  general: { tab: "general", anchor: null },
  accounts: { tab: "sources", anchor: "dirs" },
  statusline: { tab: "sources", anchor: "statusline" },
  api: { tab: "sources", anchor: "api" },
  diagnostics: { tab: "privacy", anchor: "diagnostics" },
  updates: { tab: "about", anchor: "updates" },
  widget: { tab: "general", anchor: "widget" },
};

/** The target of a URL fragment (`settings.html#api`) or a `settings-section-requested` event. */
export function targetFor(section: string | null | undefined): Target {
  // `hasOwn`: a fragment such as `#toString` must not reach the prototype.
  return section && Object.hasOwn(TARGETS, section)
    ? TARGETS[section as SettingsSection]
    : TARGETS.general;
}

/** Up to five thresholds per list, as the core keeps (notifier/prefs.rs `MAX_THRESHOLDS`). */
export const MAX_THRESHOLDS = 5;

/** A whole percentage from 1 to 100, or `null`. */
export function parseThreshold(text: string): number | null {
  const trimmed = text.trim().replace(/^%|%$/g, "");
  if (!/^\d{1,3}$/.test(trimmed)) return null;
  const n = Number(trimmed);
  return n >= 1 && n <= 100 ? n : null;
}

/** `list` with `value` added, sorted and without duplicates; `null` when it is full. */
export function addThreshold(list: readonly number[], value: number): number[] | null {
  if (list.includes(value)) return [...list];
  if (list.length >= MAX_THRESHOLDS) return null;
  return [...list, value].sort((a, b) => a - b);
}

export function removeThreshold(list: readonly number[], value: number): number[] {
  return list.filter((v) => v !== value);
}

/** `presets` plus `current` when the user set another value in `config.toml`, ascending. */
export function optionsWith(presets: readonly number[], current: number): number[] {
  return presets.includes(current) ? [...presets] : [...presets, current].sort((a, b) => a - b);
}

export const WARM_TTL_PRESETS = [0, 30, 60, 120, 300, 600] as const;
export const FINAL_WARNING_PRESETS = [0, 5, 10, 15, 20, 30] as const;
export const RESET_PRESETS = [0, 50, 60, 70, 80, 90, 100] as const;
export const RETENTION_PRESETS = [7, 30, 90, 180, 365] as const;

const KB = 1024;
const MB = KB * 1024;
const GB = MB * 1024;

/** A file size in the largest unit that keeps it at least 1. */
export function sizeMessage(bytes: number): Message {
  if (bytes >= GB) return { key: "size-gb", args: { n: bytes / GB } };
  if (bytes >= MB) return { key: "size-mb", args: { n: bytes / MB } };
  if (bytes >= KB) return { key: "size-kb", args: { n: Math.round(bytes / KB) } };
  return { key: "size-bytes", args: { n: bytes } };
}

/** The usage API's state lines under the consent text. */
export function apiStatus(api: ApiConsentView, now: number): Message[] {
  const status = api.status;
  if (!api.granted) return [];
  if (!status) return [{ key: "api-status-unknown" }];
  const s = status.state;
  const lines: Message[] = [];
  const left = (at: number) => ({ ms: Math.max(0, at - now) });
  switch (s.state) {
    case "off":
      lines.push({ key: "api-state-off" });
      break;
    case "active":
      lines.push({ key: "api-state-active" });
      break;
    case "token_stale":
      lines.push({ key: "api-state-token-stale" });
      break;
    case "backoff":
      lines.push({ key: "api-state-backoff", args: left(s.until) });
      break;
    case "offline":
      lines.push({ key: "api-state-offline", args: left(s.retry_at) });
      break;
    case "error":
      lines.push({ key: "api-state-error", args: left(s.retry_at) });
      break;
  }
  const d = status.diagnostics;
  if (d.last_success !== null) {
    lines.push({ key: "api-last-success", args: { ms: Math.max(0, now - d.last_success) } });
  }
  if (d.requests > 0) {
    lines.push({ key: "api-requests", args: { requests: d.requests, errors: d.errors } });
  }
  if (d.last_error) lines.push({ key: "api-last-error", args: { category: d.last_error } });
  return lines;
}

/** How long ago the consent was given; `null` when it is missing or unreadable. */
export function consentAge(api: ApiConsentView, now: number): number | null {
  if (!api.granted || !api.granted_at) return null;
  const at = Date.parse(api.granted_at);
  return Number.isNaN(at) ? null : Math.max(0, now - at);
}

export function statuslineErrorKey(error: StatuslineError): string {
  switch (error.error) {
    case "no_directory":
      return "statusline-error-no-directory";
    case "cli_missing":
      return "statusline-error-cli-missing";
    case "mod_missing":
      return "mod-missing";
    case "invalid_settings":
      return "statusline-error-invalid-settings";
    case "changed_meanwhile":
      return "statusline-error-changed-meanwhile";
    case "stale_plan":
      return "statusline-error-stale-plan";
    case "io":
      return "statusline-error-io";
  }
}

export function pickErrorKey(error: PickError): string {
  switch (error.error) {
    case "not_a_directory":
      return "sources-pick-not-dir";
    case "not_claude_dir":
      return "sources-pick-not-claude";
    case "io":
      return "sources-pick-failed";
  }
}

/** What About's "Updates" card shows: status lines and which buttons are live. */
export interface UpdateUi {
  lines: Message[];
  /** "Install and restart": an installed copy with a found update. */
  install: boolean;
  /** A portable copy with a found update: the releases page is the way to get it. */
  download: boolean;
  /** Checking or installing: both buttons wait. */
  busy: boolean;
}

export function updateUi(u: UpdateView, now: number): UpdateUi {
  const lines: Message[] = [];
  const s = u.status;
  const busy = s.kind === "checking" || s.kind === "installing";
  switch (s.kind) {
    case "idle":
      break;
    case "checking":
      lines.push({ key: "about-update-checking" });
      break;
    case "up_to_date":
      lines.push({ key: "about-update-current" });
      break;
    case "found":
      break;
    case "installing":
      lines.push(
        s.percent === null
          ? { key: "about-update-downloading" }
          : { key: "about-update-downloading-pct", args: { pct: s.percent } },
      );
      break;
    case "failed":
      lines.push({
        key: s.stage === "check" ? "about-update-check-failed" : "about-update-install-failed",
      });
      break;
  }
  if (u.available) {
    lines.unshift({ key: "about-update-available", args: { version: u.available.version } });
    if (!u.can_install) lines.push({ key: "about-update-portable" });
  }
  if (u.checked_at !== null && !busy) {
    lines.push({ key: "about-update-checked", args: { ms: Math.max(0, now - u.checked_at) } });
  }
  return {
    lines,
    install: u.available !== null && u.can_install && s.kind !== "installing",
    download: u.available !== null && !u.can_install,
    busy,
  };
}

// --- Statusline editor (Step 12j) -----------------------------------------------------------
//
// The editor keeps an unsaved draft of `StatuslinePrefs`. Every rule here returns a new item
// list; the core still normalizes what is saved (prefs.rs `normalized`), so these only keep the
// UI from offering what the core would drop.

/** The core's limits (statusline/line/prefs.rs). */
export const MAX_ITEMS = 64;
export const MAX_LABEL = 16;
export const MAX_TEXT = 32;
export const MAX_SEPARATOR = 3;

/** The preview's widths in terminal columns. */
export const WIDTHS = [120, 80, 50] as const;
export type Width = (typeof WIDTHS)[number];

export const SAMPLE_STATES: readonly SampleState[] = ["normal", "warn", "crit", "stale"];
export const THEMES: readonly Theme[] = ["plain", "powerline", "powerline_round"];
export const COLOR_MODES: readonly ColorMode[] = ["auto", "always", "never"];
export const ICON_SETS: readonly IconSet[] = ["text", "unicode", "nerd"];
export const GROUPS: readonly Group[] = ["limit", "session", "project", "layout"];

/** The palette of the color swatches, in the order a terminal lists them. */
export const ANSI_COLORS: readonly AnsiColor[] = [
  "black",
  "red",
  "green",
  "yellow",
  "blue",
  "magenta",
  "cyan",
  "white",
  "gray",
  "bright_red",
  "bright_green",
  "bright_yellow",
  "bright_blue",
  "bright_magenta",
  "bright_cyan",
  "bright_white",
];

const LAYOUT: readonly SegmentKind[] = ["separator", "line_break", "custom_text", "space", "flex"];

/** Kinds whose warning and critical colors win over the user's color (ADR-049). */
const THRESHOLD_COLORED: readonly SegmentKind[] = [
  "five_hour",
  "seven_day",
  "scoped",
  "spend_limit",
  "context",
];

/** Kinds with a reset countdown and a time mark. */
const WINDOWS: readonly SegmentKind[] = ["five_hour", "seven_day"];

const kebab = (s: string) => s.replaceAll("_", "-");

export const kindKey = (kind: SegmentKind) => `statusline-kind-${kebab(kind)}`;
export const kindHintKey = (kind: SegmentKind) => `statusline-kind-${kebab(kind)}-hint`;
export const formatKey = (format: Format) => `statusline-format-${kebab(format)}`;
export const colorKey = (color: AnsiColor) => `statusline-color-${kebab(color)}`;
export const groupKey = (group: Group) => `statusline-group-${group}`;
export const themeKey = (theme: Theme) => `statusline-theme-${kebab(theme)}`;
export const colorModeKey = (mode: ColorMode) => `statusline-color-mode-${mode}`;
export const stateKey = (state: SampleState) => `statusline-state-${state}`;
export const resetFormatKey = (f: ResetFormat) => `statusline-reset-format-${f}`;
export const barStyleKey = (style: BarStyle) => `statusline-bar-style-${style}`;
export const iconSetKey = (set: IconSet) => `statusline-icons-${set}`;

/**
 * A bar sample's spans as glyph runs for the style picker. Only the solid bar's backgrounds (and
 * the text on them) bring their terminal colors; the other styles read by shape in the control's
 * own text color.
 */
export function glyphRuns(spans: readonly Span[]): GlyphRun[] {
  return spans.map((s) => ({
    text: s.text,
    color: s.bg && s.fg ? `var(--term-${kebab(s.fg)})` : null,
    background: s.bg ? `var(--term-${kebab(s.bg)})` : null,
  }));
}

/**
 * What the installed Nerd Font (Step 12o) means for the Nerd icons or the capsule: a message key,
 * or `null` when there is nothing to say. `loaded` false (the font state is not known yet) says
 * nothing rather than a warning that may be wrong.
 */
export function fontNoteKey(
  need: "icons" | "capsule",
  font: NerdFontView | null,
  loaded: boolean,
): string | null {
  if (!loaded) return null;
  const tier = font?.tier ?? "none";
  if (need === "capsule") {
    if (tier === "none") return "statusline-capsule-no-font";
    return tier === "old" ? "statusline-capsule-old-font" : null;
  }
  if (tier === "none") return "statusline-icons-no-font";
  return font && !font.mono ? "statusline-icons-not-mono" : null;
}

/** The bar widths in cells (prefs.rs `BAR_WIDTHS`). */
export const BAR_WIDTHS = [4, 8, 12] as const;

export const isLayout = (kind: SegmentKind) => LAYOUT.includes(kind);
export const hasThresholdColors = (kind: SegmentKind) => THRESHOLD_COLORED.includes(kind);
export const isWindow = (kind: SegmentKind) => WINDOWS.includes(kind);

/**
 * A new item of `kind` with its defaults; the core fills in the format. A separator gets
 * `separator`, the toolbar's glyph for new separators (ADR-051); blank: the core's default.
 */
export function newItem(kind: SegmentKind, separator = ""): Item {
  return {
    kind,
    enabled: true,
    format: null,
    label: "",
    color: null,
    reset: true,
    pace: true,
    text: kind === "separator" && separator.trim() !== "" ? separator : "",
    width: 1,
    reset_format: "countdown",
    bar_width: 8,
    bar_style: "blocks",
  };
}

/** How many items draw a value (not layout items). */
export function shownCount(items: readonly Item[]): number {
  return items.filter((i) => i.enabled && !isLayout(i.kind)).length;
}

/**
 * Whether the item at `index` may be hidden or removed: the core falls back to the default line
 * when no value item is left (prefs.rs `normalized`), so the last one stays.
 */
export function canDrop(items: readonly Item[], index: number): boolean {
  const item = items[index];
  if (!item) return false;
  return isLayout(item.kind) || !item.enabled || shownCount(items) > 1;
}

/** `items` with the item at `index` hidden or shown again; unchanged when it may not hide. */
export function toggleHidden(items: readonly Item[], index: number): Item[] {
  const item = items[index];
  if (!item || (item.enabled && !canDrop(items, index))) return [...items];
  return items.map((i, n) => (n === index ? { ...i, enabled: !i.enabled } : i));
}

export function removeItem(items: readonly Item[], index: number): Item[] {
  return canDrop(items, index) ? items.filter((_, n) => n !== index) : [...items];
}

/** `items` with the item at `from` moved so it ends up at `to`. */
export function moveItem(items: readonly Item[], from: number, to: number): Item[] {
  const list = [...items];
  if (from < 0 || from >= list.length || to < 0 || to >= list.length || from === to) return list;
  const [item] = list.splice(from, 1);
  list.splice(to, 0, item as Item);
  return list;
}

/**
 * Where a dragged row lands: `slot` is the gap it was dropped in (0 = before the first row,
 * `length` = after the last); the result is the index `moveItem` takes.
 */
export function dropIndex(from: number, slot: number): number {
  return slot > from ? slot - 1 : slot;
}

/** Whether `kind` can be added: layout items any number of times, value items once. */
export function canAdd(items: readonly Item[], kind: SegmentKind): boolean {
  if (items.length >= MAX_ITEMS) return false;
  return isLayout(kind) || !items.some((i) => i.kind === kind);
}

/** A value item: not a layout item (a hidden one counts too; the core drops its separator). */
const isValue = (item: Item | undefined) => !!item && !isLayout(item.kind);

/**
 * `items` with a new `kind` after the item at `after` (at the end when `null`), and its index;
 * `null` when it cannot be added. A new value next to another value in its line gets a
 * separator between them (Step 12l): before it, and after it when it lands in front of one.
 */
export function addItem(
  items: readonly Item[],
  kind: SegmentKind,
  after: number | null,
  separator = "",
): { items: Item[]; index: number } | null {
  if (!canAdd(items, kind)) return null;
  let index = after === null || after >= items.length ? items.length : Math.max(0, after + 1);
  const list = [...items];
  const value = !isLayout(kind);
  // A line break never neighbors a value, so the check stays inside the line.
  const before = value && isValue(list[index - 1]);
  const behind = value && isValue(list[index]);
  const room = () => list.length < MAX_ITEMS;
  list.splice(index, 0, newItem(kind, separator));
  if (behind && room()) list.splice(index + 1, 0, newItem("separator", separator));
  if (before && room()) {
    list.splice(index, 0, newItem("separator", separator));
    index += 1;
  }
  return { items: list, index };
}

// --- Lines (Step 12l) ------------------------------------------------------------------------
//
// The list stays flat; the editor's line cards are a view of it, split at the line breaks the
// core draws (enabled ones, render.rs `spans`). A hidden line break is an item of its card.

const isBreak = (item: Item | undefined) => !!item && item.kind === "line_break" && item.enabled;

export interface Line {
  /** The indices of the card's items, line breaks that split the list left out. */
  items: number[];
  /** The line break that ends the card; `null` on the last one. */
  end: number | null;
}

/** The line cards: never empty, and an empty card for two line breaks in a row. */
export function lines(items: readonly Item[]): Line[] {
  const out: Line[] = [{ items: [], end: null }];
  items.forEach((item, i) => {
    const last = out[out.length - 1] as Line;
    if (isBreak(item)) {
      last.end = i;
      out.push({ items: [], end: null });
    } else {
      last.items.push(i);
    }
  });
  return out;
}

/** The card the item at `index` is in, its 0-based position there and the card's size. */
export function placeOf(
  items: readonly Item[],
  index: number,
): { line: number; position: number; count: number } | null {
  const all = lines(items);
  for (const [line, l] of all.entries()) {
    const position = l.items.indexOf(index);
    if (position >= 0) return { line, position, count: l.items.length };
  }
  return null;
}

/** Where an item added to card `line` goes: after `after` when it is in the card, else last. */
function slotIn(all: Line[], line: number, length: number, after: number | null): number {
  const l = all[line];
  if (!l) return length;
  if (after !== null && l.items.includes(after)) return after + 1;
  return l.end ?? length;
}

/** `addItem` into card `line`: after the selection when it is there, at its end otherwise. */
export function addToLine(
  items: readonly Item[],
  kind: SegmentKind,
  line: number,
  after: number | null,
  separator = "",
): { items: Item[]; index: number } | null {
  return addItem(items, kind, slotIn(lines(items), line, items.length, after) - 1, separator);
}

/** `items` with a new, empty card at the end; `null` when the list is full. */
export function addLine(items: readonly Item[]): Item[] | null {
  if (items.length >= MAX_ITEMS) return null;
  return [...items, newItem("line_break")];
}

/** The items card `line` takes away with it: its own and the line break that bounds it. */
function lineRange(all: Line[], line: number, length: number): { start: number; count: number } {
  const l = all[line] as Line;
  // The first card ends at its line break; any other starts at the previous card's one.
  const start = line === 0 ? 0 : (all[line - 1]?.end as number);
  const end = line === 0 ? (l.end as number) + 1 : (l.end ?? length);
  return { start, count: end - start };
}

/** Whether card `line` may go: not the only one, and not with the last value item shown. */
export function canRemoveLine(items: readonly Item[], line: number): boolean {
  const all = lines(items);
  if (all.length < 2 || !all[line]) return false;
  const { start, count } = lineRange(all, line, items.length);
  const rest = items.filter((_, n) => n < start || n >= start + count);
  return shownCount(rest) > 0;
}

/**
 * `items` without card `line` and its items; `start` and `count` tell which indices went, so
 * the selection can follow. `null` when the card may not go.
 */
export function removeLine(
  items: readonly Item[],
  line: number,
): { items: Item[]; start: number; count: number } | null {
  if (!canRemoveLine(items, line)) return null;
  const { start, count } = lineRange(lines(items), line, items.length);
  return { items: items.filter((_, n) => n < start || n >= start + count), start, count };
}

/**
 * `items` with the item at `from` moved into card `line` at `position` (of the card without
 * it; clamped, so `Infinity` is last), and its new index. A line break that splits the list
 * does not move this way.
 */
export function moveToLine(
  items: readonly Item[],
  from: number,
  line: number,
  position: number,
): { items: Item[]; index: number } | null {
  const item = items[from];
  if (!item || isBreak(item)) return null;
  const rest = items.filter((_, n) => n !== from);
  const l = lines(rest)[line];
  if (!l) return null;
  const at = Math.max(0, Math.min(position, l.items.length));
  const index = l.items[at] ?? l.end ?? rest.length;
  rest.splice(index, 0, item);
  return { items: rest, index };
}

/**
 * A drop of the item at `from` into gap `slot` of card `line` (0 = before its first row). In
 * its own card the gap is counted with the item still there, as `dropIndex` does.
 */
export function dropInto(
  items: readonly Item[],
  from: number,
  line: number,
  slot: number,
): { items: Item[]; index: number } | null {
  const place = placeOf(items, from);
  if (!place) return null;
  const position = place.line === line ? dropIndex(place.position, slot) : slot;
  if (place.line === line && position === place.position) return null;
  return moveToLine(items, from, line, position);
}

/** What a flexible space does, by how many the card shows (ADR-051). */
export function flexHintKey(items: readonly Item[], index: number): string {
  const place = placeOf(items, index);
  const card = place ? (lines(items)[place.line]?.items ?? []) : [];
  const n = card.filter((i) => items[i]?.kind === "flex" && items[i]?.enabled).length;
  if (n >= 3) return "statusline-flex-spread";
  return n === 2 ? "statusline-flex-center" : "statusline-flex-right";
}

/** Every separator with `glyph`, which new separators take too. */
export function applySeparator(prefs: StatuslinePrefs, glyph: string): StatuslinePrefs {
  return {
    ...prefs,
    separator: glyph,
    items: prefs.items.map((i) => (i.kind === "separator" ? { ...i, text: glyph } : i)),
  };
}

/** The widths a space offers and the right margins, in columns (prefs.rs `MAX_SPACE`, `MAX_MARGIN`). */
export const SPACE_WIDTHS = [1, 2, 3, 4, 5, 6, 7, 8] as const;
export const MARGINS = [0, 1, 2, 3, 4, 5, 6, 7, 8] as const;
/** The columns Claude Code keeps free at the right (render.rs `CLAUDE_CODE_RESERVED`). */
export const RESERVED_COLUMNS = 4;

export function updateItem(items: readonly Item[], index: number, patch: Partial<Item>): Item[] {
  return items.map((i, n) => (n === index ? { ...i, ...patch } : i));
}

/** The row to select after the one at `index` was removed from a list of `length` rows. */
export function selectionAfterRemove(length: number, index: number): number | null {
  if (length <= 1) return null;
  return Math.min(index, length - 2);
}

export interface PaletteEntry {
  kind: SegmentKind;
  sample: string;
  /** A value item already in the line. */
  used: boolean;
  /** The account is known not to have it (ADR-041). */
  absent: boolean;
}

export interface PaletteGroup {
  group: Group;
  entries: PaletteEntry[];
}

/**
 * The layout group's order in the palette. A line break is not offered: "Add line" under the
 * cards makes one.
 */
const PALETTE_LAYOUT: readonly SegmentKind[] = ["separator", "space", "flex", "custom_text"];

const paletteOrder = (kind: SegmentKind) => {
  const n = PALETTE_LAYOUT.indexOf(kind);
  return n < 0 ? -1 : n;
};

/** The add-item palette: the catalog in groups, in catalog order (layout: `PALETTE_LAYOUT`). */
export function palette(
  catalog: readonly StatuslineKind[],
  items: readonly Item[],
): PaletteGroup[] {
  return GROUPS.map((group) => ({
    group,
    entries: catalog
      .filter((k) => k.group === group && k.kind !== "line_break")
      .sort((a, b) => paletteOrder(a.kind) - paletteOrder(b.kind))
      .map((k) => ({
        kind: k.kind,
        sample: k.sample,
        used: !isLayout(k.kind) && items.some((i) => i.kind === k.kind),
        absent: k.absent,
      })),
  })).filter((g) => g.entries.length > 0);
}

/** What the row of the item at `index` shows: its text in the preview, if drawn there. */
export function itemText(preview: StatuslinePreview, index: number): string | null {
  const text = preview.lines
    .flat()
    .filter((s) => s.item === index)
    .map((s) => s.text)
    .join("")
    .trim();
  return text === "" ? null : text;
}

/**
 * An item's preview text as glyph runs when part of it is drawn by background (a solid bar,
 * Step 12p), so the list shows the colored cells rather than blank space; otherwise `null` and
 * the plain `itemText` stands. The ends are trimmed as there, but never a colored cell.
 */
export function itemRuns(preview: StatuslinePreview, index: number): GlyphRun[] | null {
  const spans = preview.lines.flat().filter((s) => s.item === index);
  if (!spans.some((s) => s.bg)) return null;
  const runs = glyphRuns(spans);
  const first = runs[0];
  const last = runs[runs.length - 1];
  if (first && !first.background) first.text = first.text.trimStart();
  if (last && !last.background) last.text = last.text.trimEnd();
  return runs.filter((r) => r.text !== "");
}

/**
 * The CSS classes of a preview span: the user's (or a Powerline block's) color, otherwise the
 * paint's; `p-dim` adds the terminal's faint style (SGR 2) on top, as the CLI does.
 */
export function spanClasses(span: Span): string {
  const classes: string[] = [];
  if (span.fg) classes.push(`fg-${kebab(span.fg)}`);
  else if (span.paint !== "plain" && span.paint !== "dim") classes.push(`p-${span.paint}`);
  if (span.paint === "dim") classes.push("p-dim");
  if (span.bg) classes.push(`bg-${kebab(span.bg)}`);
  return classes.join(" ");
}

export function samePrefs(a: StatuslinePrefs, b: StatuslinePrefs): boolean {
  return JSON.stringify(a) === JSON.stringify(b);
}

// --- Undo ------------------------------------------------------------------------------------

/** Steps kept; the history lives as long as the window and is never written to disk. */
export const HISTORY_LIMIT = 100;
/** Edits of one field closer together than this are undone at once (typing a label). */
export const COALESCE_MS = 1000;

export interface History {
  past: StatuslinePrefs[];
  /** What the last step changed (`label:3`); `null` for steps that never coalesce. */
  key: string | null;
  at: number;
}

export const emptyHistory = (): History => ({ past: [], key: null, at: 0 });

/**
 * `history` after a change from `before`. A change of the same field (`key`) within
 * `COALESCE_MS` of the previous one joins that step, so one undo takes back a whole word.
 */
export function record(
  history: History,
  before: StatuslinePrefs,
  key: string | null,
  now: number,
): History {
  if (key !== null && key === history.key && now - history.at < COALESCE_MS) {
    return { ...history, at: now };
  }
  const past = [...history.past, before].slice(-HISTORY_LIMIT);
  return { past, key, at: now };
}

/** The previous prefs and the history without them; `null` when there is nothing to undo. */
export function undo(history: History): { prefs: StatuslinePrefs; history: History } | null {
  const prefs = history.past.at(-1);
  if (!prefs) return null;
  return { prefs, history: { past: history.past.slice(0, -1), key: null, at: 0 } };
}

// --- Debounced saving -----------------------------------------------------------------------

/** How long the editor waits after the last change before saving. */
export const SAVE_DELAY_MS = 400;

export interface Timers {
  set: (f: () => void, ms: number) => unknown;
  clear: (id: unknown) => void;
}

const realTimers: Timers = {
  set: (f, ms) => setTimeout(f, ms),
  clear: (id) => clearTimeout(id as ReturnType<typeof setTimeout>),
};

export interface Saver<T> {
  /** Saves `value` after the delay; a later call replaces it and restarts the wait. */
  schedule(value: T): void;
  /** Saves a waiting value now; resolves once every save has finished. */
  flush(): Promise<void>;
  /** A value is waiting or being saved. */
  pending(): boolean;
}

/**
 * Coalesces rapid changes into one save and never runs two saves at once, so the core always
 * ends with the last value. `save` reports success; a failure is the caller's to show.
 */
export function createSaver<T>(
  delay: number,
  save: (value: T) => Promise<unknown>,
  timers: Timers = realTimers,
): Saver<T> {
  let waiting: { value: T } | null = null;
  let timer: unknown = null;
  let running: Promise<void> | null = null;

  const run = (): Promise<void> => {
    if (running) return running;
    running = (async () => {
      while (waiting) {
        const { value } = waiting;
        waiting = null;
        await save(value);
      }
      running = null;
    })();
    return running;
  };

  return {
    schedule(value) {
      waiting = { value };
      if (timer !== null) timers.clear(timer);
      timer = timers.set(() => {
        timer = null;
        void run();
      }, delay);
    },
    async flush() {
      if (timer !== null) {
        timers.clear(timer);
        timer = null;
      }
      await run();
    },
    pending: () => waiting !== null || running !== null,
  };
}

/** The widget's background opacity steps, the same as its context menu (widget/system.rs). */
export const WIDGET_OPACITY_STEPS = [100, 85, 70, 55, 40] as const;

/** The taskbar offsets offered, in logical pixels; a drag on the taskbar sets any value. */
export const TASKBAR_OFFSET_PRESETS = [0, 40, 80, 160, 320, 640] as const;

/** The keys of a keyboard event the shortcut recorder reads. */
export interface KeyPress {
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
  /** The physical key (`KeyR`, `Digit1`, `F5`), so the keyboard layout does not matter. */
  code: string;
}

/**
 * A key press as a global shortcut, written like the core's `Hotkey` (`Ctrl+Alt+Shift+R`).
 * `pending` while only modifiers are down; `invalid` without Ctrl, Alt or Win, or for a key the
 * core does not take (letters, digits and F1–F24 only).
 */
export function hotkeyFromKey(
  e: KeyPress,
): { kind: "pending" } | { kind: "invalid" } | { kind: "ok"; text: string } {
  if (/^(Control|Alt|Shift|Meta|OS)(Left|Right)?$/.test(e.code)) return { kind: "pending" };
  const key =
    /^Key([A-Z])$/.exec(e.code)?.[1] ??
    /^Digit([0-9])$/.exec(e.code)?.[1] ??
    /^(F(?:[1-9]|1[0-9]|2[0-4]))$/.exec(e.code)?.[1];
  if (!key || !(e.ctrlKey || e.altKey || e.metaKey)) return { kind: "invalid" };
  const mods = [
    e.ctrlKey && "Ctrl",
    e.altKey && "Alt",
    e.shiftKey && "Shift",
    e.metaKey && "Win",
  ].filter(Boolean);
  return { kind: "ok", text: [...mods, key].join("+") };
}

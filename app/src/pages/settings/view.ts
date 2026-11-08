// The settings window's pure logic: which tab a panel link opens, how sizes and API states are
// worded, and the rules for threshold chips. Components only render what these return.

import type { Args } from "../../lib/i18n/fluent";
import type {
  AnsiColor,
  ApiConsentView,
  ColorMode,
  Format,
  Group,
  Item,
  PickError,
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
const THRESHOLD_COLORED: readonly SegmentKind[] = ["five_hour", "seven_day", "scoped", "context"];

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

/**
 * `items` with a new `kind` after the item at `after` (at the end when `null`), and its index;
 * `null` when it cannot be added.
 */
export function addItem(
  items: readonly Item[],
  kind: SegmentKind,
  after: number | null,
  separator = "",
): { items: Item[]; index: number } | null {
  if (!canAdd(items, kind)) return null;
  const index = after === null || after >= items.length ? items.length : after + 1;
  const list = [...items];
  list.splice(index, 0, newItem(kind, separator));
  return { items: list, index };
}

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

/** The add-item palette: the catalog in groups, in catalog order. */
export function palette(
  catalog: readonly StatuslineKind[],
  items: readonly Item[],
): PaletteGroup[] {
  return GROUPS.map((group) => ({
    group,
    entries: catalog
      .filter((k) => k.group === group)
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

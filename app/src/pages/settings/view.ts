// The settings window's pure logic: which tab a panel link opens, how sizes and API states are
// worded, and the rules for threshold chips. Components only render what these return.

import type { Args } from "../../lib/i18n/fluent";
import type {
  ApiConsentView,
  Item,
  PickError,
  SegmentKind,
  SettingsSection,
  StatuslineError,
  UpdateView,
} from "../../lib/ipc/bindings";

export type Tab = "general" | "sources" | "notifications" | "privacy" | "about";

export const TABS: readonly Tab[] = ["general", "sources", "notifications", "privacy", "about"];

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

/**
 * The segments the 12c picker offers, in the default order. The core knows more kinds
 * (`statusline_catalog`); the Step 12j editor offers them all.
 */
export const SEGMENTS = [
  "five_hour",
  "seven_day",
  "scoped",
  "context",
  "session_share",
  "cost",
  "model",
] as const satisfies readonly SegmentKind[];

/** The windows the bar and time-mark switches act on. */
const WINDOWS: readonly SegmentKind[] = ["five_hour", "seven_day"];

/** The Fluent key naming a segment in the picker. */
export function segmentLabelKey(kind: SegmentKind): string {
  return `statusline-segment-${kind.replaceAll("_", "-")}`;
}

/** A new item of `kind` with its defaults; the core fills in the format. */
export function newItem(kind: SegmentKind): Item {
  return {
    kind,
    enabled: true,
    format: null,
    label: "",
    color: null,
    reset: true,
    pace: true,
    text: "",
  };
}

export interface SegmentRow {
  kind: SegmentKind;
  enabled: boolean;
  /** Position among the enabled rows; `null` when off. */
  index: number | null;
}

function picked(items: readonly Item[]): Item[] {
  return items.filter((i) => i.enabled && (SEGMENTS as readonly SegmentKind[]).includes(i.kind));
}

/** The rows of the segment picker: the enabled ones in their order, then the rest. */
export function segmentRows(items: readonly Item[]): SegmentRow[] {
  const on = picked(items).map((i, index) => ({ kind: i.kind, enabled: true, index }));
  const off = SEGMENTS.filter((k) => !on.some((r) => r.kind === k)).map((kind) => ({
    kind,
    enabled: false,
    index: null,
  }));
  return [...on, ...off];
}

/** How many items draw a value (not layout items). */
export function shownCount(items: readonly Item[]): number {
  return items.filter((i) => i.enabled && !LAYOUT.includes(i.kind)).length;
}

const LAYOUT: readonly SegmentKind[] = ["separator", "line_break", "custom_text"];

/**
 * `items` with `kind` switched on (appended when missing) or off (kept, with its settings).
 * The last item that draws a value cannot be switched off.
 */
export function toggleSegment(items: readonly Item[], kind: SegmentKind, on: boolean): Item[] {
  if (on) {
    return items.some((i) => i.kind === kind)
      ? items.map((i) => (i.kind === kind ? { ...i, enabled: true } : i))
      : [...items, newItem(kind)];
  }
  const target = items.find((i) => i.kind === kind && i.enabled);
  if (!target || shownCount(items) <= 1) return [...items];
  return items.map((i) => (i === target ? { ...i, enabled: false } : i));
}

/** `items` with the picker row at `index` swapped with the one above (`-1`) or below (`1`). */
export function moveSegment(items: readonly Item[], index: number, by: -1 | 1): Item[] {
  const rows = picked(items);
  const a = rows[index];
  const b = rows[index + by];
  if (!a || !b) return [...items];
  return items.map((i) => (i === a ? b : i === b ? a : i));
}

/** Whether the line has a second line (the 12c double layout). */
export function isDouble(items: readonly Item[]): boolean {
  return items.some((i) => i.kind === "line_break" && i.enabled);
}

/** `items` on one line, or with a line break after the last limit window (12c `double`). */
export function setDouble(items: readonly Item[], double: boolean): Item[] {
  const flat = items.filter((i) => i.kind !== "line_break");
  if (!double) return flat;
  let at = -1;
  flat.forEach((i, n) => {
    if (i.enabled && (WINDOWS.includes(i.kind) || i.kind === "scoped")) at = n;
  });
  if (at < 0 || at === flat.length - 1) return [...items];
  return [...flat.slice(0, at + 1), newItem("line_break"), ...flat.slice(at + 1)];
}

/** Whether the limit windows draw a bar. */
export function barOn(items: readonly Item[]): boolean {
  return items.filter((i) => WINDOWS.includes(i.kind)).every((i) => i.format !== "percent");
}

export function setBar(items: readonly Item[], on: boolean): Item[] {
  return items.map((i) =>
    WINDOWS.includes(i.kind) ? { ...i, format: on ? "bar" : "percent" } : i,
  );
}

/** Whether the limit windows show the time mark. */
export function paceOn(items: readonly Item[]): boolean {
  return items.filter((i) => WINDOWS.includes(i.kind)).every((i) => i.pace);
}

export function setPace(items: readonly Item[], on: boolean): Item[] {
  return items.map((i) => (WINDOWS.includes(i.kind) ? { ...i, pace: on } : i));
}

// The settings window's pure logic: which tab a panel link opens, how sizes and API states are
// worded, and the rules for threshold chips. Components only render what these return.

import type { Args } from "../../lib/i18n/fluent";
import type {
  ApiConsentView,
  PickError,
  SettingsSection,
  StatuslineError,
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

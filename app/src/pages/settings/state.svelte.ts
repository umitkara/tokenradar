// The settings window's state. Preferences arrive with `get_prefs` and stay current through
// `prefs-changed`; data sources and storage are reloaded when the window becomes visible, after
// each action, and (throttled) when the core reports new limits, so the statusline's "last
// reading" and the API state do not go stale while the window is open.

import {
  type AboutView,
  type CoreEvent,
  commands,
  type DataSourcesView,
  events,
  type PrefsView,
  type StorageView,
} from "../../lib/ipc/bindings";
import { subscribeCoreEvents } from "../../lib/ipc/core-events";
import { inTauri } from "../../lib/ipc/runtime";

/** At most one data-source reload per this many milliseconds from core events. */
const SOURCES_THROTTLE_MS = 10_000;

export const store = $state<{
  prefs: PrefsView | null;
  sources: DataSourcesView | null;
  storage: StorageView | null;
  about: AboutView | null;
  /** `null` while unknown or unreadable. */
  autostart: boolean | null;
}>({ prefs: null, sources: null, storage: null, about: null, autostart: null });

let lastSources = 0;

export async function loadSources(): Promise<void> {
  if (!inTauri()) return;
  lastSources = Date.now();
  const r = await commands.getDataSources();
  if (r.status === "ok") store.sources = r.data;
  else console.error("settings: data sources", r.error);
}

export async function loadStorage(): Promise<void> {
  if (!inTauri()) return;
  const r = await commands.getStorage();
  if (r.status === "ok") store.storage = r.data;
  else console.error("settings: storage", r.error);
}

async function loadAutostart(): Promise<void> {
  const r = await commands.getAutostart();
  store.autostart = r.status === "ok" ? r.data : null;
}

/** Everything; called on start and whenever the window becomes visible. */
export async function loadAll(): Promise<void> {
  if (!inTauri()) return;
  const [prefs, about] = await Promise.all([commands.getPrefs(), commands.getAbout()]);
  store.prefs = prefs;
  store.about = about;
  await Promise.all([loadSources(), loadStorage(), loadAutostart()]);
}

function onEvent(event: CoreEvent): void {
  if (event.type !== "limits_updated" && event.type !== "index_milestone") return;
  if (Date.now() - lastSources >= SOURCES_THROTTLE_MS) void loadSources();
}

/** Starts listening; returns the stop function. */
export function startSettings(): () => void {
  if (!inTauri()) return () => {};
  const offCore = subscribeCoreEvents({ onEvent, onResume: () => void loadAll() });
  const offPrefs = events.prefsChanged.listen((e) => {
    store.prefs = e.payload;
  });
  return () => {
    offCore();
    void offPrefs.then((off) => off());
  };
}

/**
 * Runs a save command that answers with the new preferences. Returns `false` on failure; the
 * window then shows `settings-save-failed`.
 */
export async function savePrefs(
  run: () => Promise<{ status: "ok"; data: PrefsView } | { status: "error"; error: string }>,
): Promise<boolean> {
  const r = await run();
  if (r.status === "ok") {
    store.prefs = r.data;
    return true;
  }
  console.error("settings: save failed", r.error);
  return false;
}

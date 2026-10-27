// The panel's state: the last snapshot and the notification mute. Loaded on start and every
// time the panel becomes visible again, then kept current by core events while visible
// (Step 10 technical notes: a missed event means a full reload, never a patch).

import { type CoreEvent, commands, events, type PanelSnapshot } from "../../lib/ipc/bindings";
import { subscribeCoreEvents } from "../../lib/ipc/core-events";
import { inTauri } from "../../lib/ipc/runtime";

export const panel = $state<{
  snapshot: PanelSnapshot | null;
  /** The first load finished (with or without a snapshot). */
  loaded: boolean;
  muted: boolean;
}>({ snapshot: null, loaded: false, muted: false });

let loading: Promise<void> | null = null;

/** Fetches the full state; concurrent calls share one request. */
export function load(): Promise<void> {
  if (!inTauri()) {
    panel.loaded = true;
    return Promise.resolve();
  }
  loading ??= Promise.all([commands.getPanelSnapshot(), commands.getMuted()])
    .then(([snapshot, muted]) => {
      panel.snapshot = snapshot;
      panel.muted = muted;
    })
    .catch((error: unknown) => console.error("panel: load failed", error))
    .finally(() => {
      panel.loaded = true;
      loading = null;
    });
  return loading;
}

function onEvent(event: CoreEvent): void {
  switch (event.type) {
    case "limits_updated":
      panel.snapshot = event.data;
      break;
    case "index_progress":
      // Raw progress between snapshots; the snapshot carries the rest of the index state.
      if (panel.snapshot && !panel.snapshot.index_state.initial_scan_done) {
        panel.snapshot.index_state.progress = event.data.progress;
      }
      break;
    case "index_milestone":
    case "schema_drift":
      void load();
      break;
    default:
      // Live sessions and usage reach the panel as `limits_updated` once the core rebuilt it.
      break;
  }
}

/** Starts listening; returns the stop function. */
export function startPanel(): () => void {
  const stopCore = subscribeCoreEvents({ onEvent, onResume: () => void load() });
  if (!inTauri()) return stopCore;
  const offMute = events.muteChanged.listen((e) => {
    panel.muted = e.payload.muted;
  });
  return () => {
    stopCore();
    void offMute.then((off) => off());
  };
}

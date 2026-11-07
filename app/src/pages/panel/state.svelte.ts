// The panel's state: the last snapshot, the notification mute, the view (summary or detail)
// and the detail view's data. Loaded on start and every time the panel becomes visible again,
// then kept current by core events while visible (Step 10 technical notes: a missed event
// means a full reload, never a patch).

import {
  type CoreEvent,
  commands,
  events,
  type PanelDetail,
  type PanelSnapshot,
  type PanelView,
} from "../../lib/ipc/bindings";
import { subscribeCoreEvents } from "../../lib/ipc/core-events";
import { inTauri } from "../../lib/ipc/runtime";

export const panel = $state<{
  snapshot: PanelSnapshot | null;
  /** The first load finished (with or without a snapshot). */
  loaded: boolean;
  muted: boolean;
  view: PanelView;
  /** The detail view's pace, strip and chart; fetched while that view is shown. */
  detail: PanelDetail | null;
}>({ snapshot: null, loaded: false, muted: false, view: "summary", detail: null });

/** The detail view is fetched again this often while shown: the pace moves with the clock. */
export const DETAIL_REFRESH_MS = 60_000;
/** At most one detail fetch per this interval when snapshots arrive in a burst. */
const DETAIL_MIN_GAP_MS = 5_000;

let loading: Promise<void> | null = null;

/** Fetches the full state; concurrent calls share one request. */
export function load(): Promise<void> {
  if (!inTauri()) {
    panel.loaded = true;
    return Promise.resolve();
  }
  loading ??= Promise.all([
    commands.getPanelSnapshot(),
    commands.getMuted(),
    commands.getPanelView(),
  ])
    .then(([snapshot, muted, view]) => {
      panel.snapshot = snapshot;
      panel.muted = muted;
      panel.view = view;
      if (view === "detail") void loadDetail(true);
    })
    .catch((error: unknown) => console.error("panel: load failed", error))
    .finally(() => {
      panel.loaded = true;
      loading = null;
    });
  return loading;
}

let detailLoading: Promise<void> | null = null;
let detailAt = 0;

/** Fetches the detail view's data; `force` skips the burst guard. */
export function loadDetail(force = false): Promise<void> {
  if (!inTauri()) return Promise.resolve();
  if (!force && Date.now() - detailAt < DETAIL_MIN_GAP_MS) return Promise.resolve();
  detailAt = Date.now();
  detailLoading ??= commands
    .getPanelDetail()
    .then((detail) => {
      panel.detail = detail;
    })
    .catch((error: unknown) => console.error("panel: detail failed", error))
    .finally(() => {
      detailLoading = null;
    });
  return detailLoading;
}

/** Switches the view and remembers it for the next time the panel opens. */
export function setView(view: PanelView): void {
  if (panel.view === view) return;
  panel.view = view;
  if (view === "detail") void loadDetail(true);
  if (inTauri()) {
    void commands.setPanelView(view).then((r) => {
      if (r.status === "error") console.error("panel: view not saved", r.error);
    });
  }
}

function onEvent(event: CoreEvent): void {
  switch (event.type) {
    case "limits_updated":
      panel.snapshot = event.data;
      if (panel.view === "detail") void loadDetail();
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

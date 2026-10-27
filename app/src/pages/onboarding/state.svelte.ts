// Onboarding's own state: the panel snapshot (for the indexing ring) and the tray icon's pin
// state. Data sources, preferences and autostart come from the settings window's store, which
// the shared statusline dialog and consent component already use.

import {
  type CoreEvent,
  commands,
  type PanelSnapshot,
  type TrayPinState,
} from "../../lib/ipc/bindings";
import { subscribeCoreEvents } from "../../lib/ipc/core-events";
import { inTauri } from "../../lib/ipc/runtime";
import { loadSources } from "../settings/state.svelte";

export const onb = $state<{
  snapshot: PanelSnapshot | null;
  pin: TrayPinState | null;
}>({ snapshot: null, pin: null });

export async function loadSnapshot(): Promise<void> {
  if (!inTauri()) return;
  onb.snapshot = await commands.getPanelSnapshot();
}

export async function loadPin(): Promise<void> {
  if (!inTauri()) return;
  onb.pin = await commands.getTrayPin();
}

function onEvent(event: CoreEvent): void {
  switch (event.type) {
    case "index_progress":
      if (onb.snapshot && !onb.snapshot.index_state.initial_scan_done) {
        onb.snapshot.index_state.progress = event.data.progress;
      }
      break;
    case "index_milestone":
      // Counts change with each milestone; the folder list shows them.
      void loadSnapshot();
      void loadSources();
      break;
    case "limits_updated":
      onb.snapshot = event.data;
      break;
    default:
      break;
  }
}

/** Starts listening; returns the stop function. */
export function startOnboarding(): () => void {
  return subscribeCoreEvents({
    onEvent,
    onResume: () => {
      void loadSnapshot();
      void loadSources();
      void loadPin();
    },
  });
}

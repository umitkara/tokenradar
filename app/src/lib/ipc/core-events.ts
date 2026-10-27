// Core events for a page, only while it is visible (Step 09). Rust emits to visible windows
// only (src-tauri/src/ipc.rs), so a hidden page would miss events anyway: it drops its listener
// when hidden and refreshes from a full snapshot when shown again.

import { type CoreEvent, events } from "./bindings";
import { inTauri } from "./runtime";

export interface CoreEventOptions {
  /** Called on each event while the page is visible. */
  onEvent: (event: CoreEvent) => void;
  /** Called when the page becomes visible (and once at start): reload the full state here. */
  onResume: () => void;
}

/** Starts listening; returns the stop function (call it from the component's teardown). */
export function subscribeCoreEvents({ onEvent, onResume }: CoreEventOptions): () => void {
  if (!inTauri()) return () => {};
  let unlisten: (() => void) | null = null;
  let stopped = false;
  let generation = 0;

  const start = () => {
    const mine = ++generation;
    void events.coreEventEmitted
      .listen((e) => onEvent(e.payload))
      .then((off) => {
        // Hidden or stopped before the listener was registered.
        if (stopped || mine !== generation) off();
        else unlisten = off;
      });
    onResume();
  };
  const stop = () => {
    generation++;
    unlisten?.();
    unlisten = null;
  };
  const onVisibility = () => (document.visibilityState === "hidden" ? stop() : start());

  document.addEventListener("visibilitychange", onVisibility);
  if (document.visibilityState !== "hidden") start();
  return () => {
    stopped = true;
    document.removeEventListener("visibilitychange", onVisibility);
    stop();
  };
}

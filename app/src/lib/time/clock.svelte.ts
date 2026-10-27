// `clock.now`: the shared ticker as a reactive value. Reading it inside a component or effect
// subscribes; the ticker stops when the last reader is gone or the page is hidden.

import { createSubscriber } from "svelte/reactivity";
import { Ticker } from "./ticker";

const ticker = new Ticker(
  {
    now: () => Date.now(),
    setTimeout: (cb, ms) => setTimeout(cb, ms),
    clearTimeout: (h) => clearTimeout(h as ReturnType<typeof setTimeout>),
  },
  {
    hidden: () => typeof document !== "undefined" && document.visibilityState === "hidden",
    onChange: (cb) => {
      if (typeof document === "undefined") return () => {};
      document.addEventListener("visibilitychange", cb);
      return () => document.removeEventListener("visibilitychange", cb);
    },
  },
);

let now = Date.now();
const subscribe = createSubscriber((update) =>
  ticker.subscribe((t) => {
    now = t;
    update();
  }),
);

export const clock = {
  /** Milliseconds since the epoch, updated once per second while read and visible. */
  get now(): number {
    subscribe();
    return now;
  },
};

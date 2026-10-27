// The one 1 Hz clock every countdown reads (Step 09). It runs only while something listens and
// the page is visible; there is never a per-component `setInterval`.

export interface Visibility {
  hidden(): boolean;
  onChange(cb: () => void): () => void;
}

export interface Timers {
  now(): number;
  setTimeout(cb: () => void, ms: number): unknown;
  clearTimeout(handle: unknown): void;
}

export class Ticker {
  #listeners = new Set<(now: number) => void>();
  #handle: unknown = null;
  #stopVisibility: (() => void) | null = null;

  constructor(
    private readonly timers: Timers,
    private readonly visibility: Visibility,
  ) {}

  /** Calls `cb` with the current time on every whole second; returns the unsubscribe. */
  subscribe(cb: (now: number) => void): () => void {
    this.#listeners.add(cb);
    if (this.#listeners.size === 1) {
      this.#stopVisibility = this.visibility.onChange(() => this.#sync(true));
      this.#sync(false);
    }
    return () => {
      this.#listeners.delete(cb);
      if (this.#listeners.size === 0) {
        this.#stopVisibility?.();
        this.#stopVisibility = null;
        this.#halt();
      }
    };
  }

  get running(): boolean {
    return this.#handle !== null;
  }

  #sync(emitNow: boolean): void {
    if (this.visibility.hidden()) {
      this.#halt();
      return;
    }
    // Back from hidden: catch up at once instead of showing a stale countdown for a second.
    if (emitNow) this.#emit();
    if (this.#handle === null) this.#schedule();
  }

  /** Aligned to the next whole second, so every countdown flips together. */
  #schedule(): void {
    const delay = 1000 - (this.timers.now() % 1000);
    this.#handle = this.timers.setTimeout(() => {
      this.#handle = null;
      this.#emit();
      this.#schedule();
    }, delay);
  }

  #halt(): void {
    if (this.#handle !== null) this.timers.clearTimeout(this.#handle);
    this.#handle = null;
  }

  #emit(): void {
    const now = this.timers.now();
    for (const cb of this.#listeners) cb(now);
  }
}

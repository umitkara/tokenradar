// The statusline editor's state: an unsaved draft of the prefs, the undo history, the selection
// and the preview. The draft is the editor's truth; it is saved after a short pause (`Saver`)
// and drawn by the core straight away, so the preview never waits for a save. A change from
// elsewhere (another window, `config.toml`) replaces the draft only while nothing is waiting
// to be saved.

import {
  commands,
  type IconSet,
  type SampleState,
  type StatuslineKind,
  type StatuslinePrefs,
  type StatuslinePreset,
  type StatuslinePreview,
} from "../../../lib/ipc/bindings";
import { inTauri } from "../../../lib/ipc/runtime";
import { savePrefs } from "../state.svelte";
import {
  createSaver,
  emptyHistory,
  type History,
  record,
  SAVE_DELAY_MS,
  samePrefs,
  undo,
  type Width,
} from "../view";

export class StatuslineEditor {
  draft = $state<StatuslinePrefs | null>(null);
  history = $state<History>(emptyHistory());
  selected = $state<number | null>(null);
  hovered = $state<number | null>(null);
  sample = $state<SampleState>("normal");
  width = $state<Width>(120);
  preview = $state<StatuslinePreview>({ lines: [], absent: [] });
  catalog = $state<StatuslineKind[]>([]);
  presets = $state<StatuslinePreset[]>([]);
  saveFailed = $state(false);

  #saver = createSaver<StatuslinePrefs>(SAVE_DELAY_MS, (prefs) => this.#save(prefs));
  #previewSeq = 0;
  #catalogSeq = 0;

  async #save(prefs: StatuslinePrefs): Promise<void> {
    if (!inTauri()) return;
    this.saveFailed = !(await savePrefs(() => commands.setStatuslinePrefs(prefs)));
  }

  /** Loads the catalog, its labels and samples in icon set `icons`; an older answer is dropped. */
  async load(icons: IconSet): Promise<void> {
    if (!inTauri()) return;
    const seq = ++this.#catalogSeq;
    const [catalog, presets] = await Promise.all([
      commands.statuslineCatalog(icons),
      commands.listStatuslinePresets(),
    ]);
    if (seq !== this.#catalogSeq) return;
    this.catalog = catalog;
    this.presets = presets;
  }

  /** Takes the stored prefs unless the user's own changes are still on their way. */
  adopt(stored: StatuslinePrefs | null): void {
    if (!stored || this.#saver.pending()) return;
    if (this.draft && samePrefs(this.draft, stored)) return;
    this.draft = stored;
    if (this.selected === null) this.selected = stored.items.length > 0 ? 0 : null;
    this.#clampSelection();
  }

  /** Replaces the draft with `next`, remembers the old one for undo and schedules a save. */
  change(next: StatuslinePrefs, key: string | null = null): void {
    if (!this.draft || samePrefs(this.draft, next)) return;
    this.history = record(this.history, this.draft, key, Date.now());
    this.draft = next;
    this.#clampSelection();
    this.#saver.schedule(next);
  }

  undo(): void {
    const back = undo(this.history);
    if (!back) return;
    this.history = back.history;
    this.draft = back.prefs;
    this.#clampSelection();
    this.#saver.schedule(back.prefs);
  }

  get canUndo(): boolean {
    return this.history.past.length > 0;
  }

  /** Saves a waiting change now (the window is hiding or the page is left). */
  flush(): Promise<void> {
    return this.#saver.flush();
  }

  /** Redraws the preview; an answer to an older request is dropped. */
  async redraw(): Promise<void> {
    const prefs = this.draft;
    if (!prefs || !inTauri()) return;
    const seq = ++this.#previewSeq;
    const preview = await commands.previewStatusline(prefs, this.sample, this.width);
    if (seq === this.#previewSeq) this.preview = preview;
  }

  kind(kind: string): StatuslineKind | undefined {
    return this.catalog.find((k) => k.kind === kind);
  }

  #clampSelection(): void {
    const n = this.draft?.items.length ?? 0;
    if (n === 0) this.selected = null;
    else if (this.selected !== null && this.selected >= n) this.selected = n - 1;
  }
}

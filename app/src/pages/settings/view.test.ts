import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it, vi } from "vitest";
import type {
  ApiConsentView,
  Item,
  SegmentKind,
  Span,
  StatuslineKind,
  StatuslinePrefs,
  UpdateView,
} from "../../lib/ipc/bindings";
import {
  ANSI_COLORS,
  addItem,
  addLine,
  addThreshold,
  addToLine,
  apiStatus,
  applySeparator,
  barStyleKey,
  COLOR_MODES,
  canAdd,
  canDrop,
  canRemoveLine,
  colorKey,
  colorModeKey,
  consentAge,
  createSaver,
  dropIndex,
  dropInto,
  emptyHistory,
  flexHintKey,
  fontNoteKey,
  formatKey,
  GROUPS,
  glyphRuns,
  groupKey,
  HISTORY_LIMIT,
  ICON_SETS,
  iconSetKey,
  itemText,
  kindHintKey,
  kindKey,
  lines,
  MAX_ITEMS,
  MAX_THRESHOLDS,
  moveItem,
  moveToLine,
  newItem,
  optionsWith,
  palette,
  parseThreshold,
  placeOf,
  record,
  removeItem,
  removeLine,
  removeThreshold,
  resetFormatKey,
  SAMPLE_STATES,
  selectionAfterRemove,
  sizeMessage,
  spanClasses,
  stateKey,
  THEMES,
  targetFor,
  themeKey,
  toggleHidden,
  undo,
  updateItem,
  updateUi,
} from "./view";

describe("targetFor", () => {
  it("maps panel sections to a tab and an anchor", () => {
    expect(targetFor("api")).toEqual({ tab: "sources", anchor: "api" });
    expect(targetFor("statusline")).toEqual({ tab: "sources", anchor: "statusline" });
    expect(targetFor("accounts")).toEqual({ tab: "sources", anchor: "dirs" });
    expect(targetFor("diagnostics")).toEqual({ tab: "privacy", anchor: "diagnostics" });
    expect(targetFor("updates")).toEqual({ tab: "about", anchor: "updates" });
  });

  it("falls back to General for anything else", () => {
    expect(targetFor("")).toEqual({ tab: "general", anchor: null });
    expect(targetFor(null)).toEqual({ tab: "general", anchor: null });
    expect(targetFor("toString")).toEqual({ tab: "general", anchor: null });
  });
});

describe("thresholds", () => {
  it("accepts whole percentages from 1 to 100", () => {
    expect(parseThreshold(" 85 ")).toBe(85);
    expect(parseThreshold("%50")).toBe(50);
    expect(parseThreshold("100")).toBe(100);
    for (const bad of ["0", "101", "7.5", "-3", "abc", ""]) expect(parseThreshold(bad)).toBeNull();
  });

  it("adds sorted without duplicates and stops at the maximum", () => {
    expect(addThreshold([75, 90], 50)).toEqual([50, 75, 90]);
    expect(addThreshold([75, 90], 90)).toEqual([75, 90]);
    const full = [10, 20, 30, 40, 50];
    expect(full).toHaveLength(MAX_THRESHOLDS);
    expect(addThreshold(full, 60)).toBeNull();
    expect(removeThreshold([75, 90, 100], 90)).toEqual([75, 100]);
  });

  it("keeps a value from config.toml among the presets", () => {
    expect(optionsWith([0, 30, 60], 45)).toEqual([0, 30, 45, 60]);
    expect(optionsWith([0, 30, 60], 30)).toEqual([0, 30, 60]);
  });
});

describe("sizeMessage", () => {
  it("picks the largest unit at least 1", () => {
    expect(sizeMessage(512)).toEqual({ key: "size-bytes", args: { n: 512 } });
    expect(sizeMessage(2048)).toEqual({ key: "size-kb", args: { n: 2 } });
    expect(sizeMessage(5 * 1024 * 1024)).toEqual({ key: "size-mb", args: { n: 5 } });
    expect(sizeMessage(3 * 1024 ** 3)).toEqual({ key: "size-gb", args: { n: 3 } });
  });
});

describe("apiStatus", () => {
  const now = 1_800_000_000_000;
  const base: ApiConsentView = {
    granted: true,
    granted_at: new Date(now - 3_600_000).toISOString(),
    outdated: false,
    status: {
      state: { state: "backoff", until: now + 120_000 },
      diagnostics: {
        requests: 12,
        errors: 1,
        last_error: "rate_limited",
        last_error_at: now - 60_000,
        last_attempt: now - 60_000,
        last_success: now - 300_000,
      },
    },
  };

  it("words the state, the last success and the counters", () => {
    expect(apiStatus(base, now).map((m) => m.key)).toEqual([
      "api-state-backoff",
      "api-last-success",
      "api-requests",
      "api-last-error",
    ]);
    expect(apiStatus(base, now)[0]?.args).toEqual({ ms: 120_000 });
  });

  it("says nothing without consent and waits for a stopped core", () => {
    expect(apiStatus({ ...base, granted: false }, now)).toEqual([]);
    expect(apiStatus({ ...base, status: null }, now)).toEqual([{ key: "api-status-unknown" }]);
  });

  it("measures the consent's age from its RFC 3339 stamp", () => {
    expect(consentAge(base, now)).toBe(3_600_000);
    expect(consentAge({ ...base, granted_at: "nonsense" }, now)).toBeNull();
    expect(consentAge({ ...base, granted: false }, now)).toBeNull();
  });
});

describe("updateUi", () => {
  const NOW = 1_800_000_000_000;
  const base: UpdateView = {
    status: { kind: "idle" },
    available: null,
    current_version: "0.1.0",
    checked_at: null,
    can_install: true,
  };
  const found = { version: "0.1.1", notes: "Fixes", date: "2026-11-02" };

  it("shows nothing before the first check", () => {
    expect(updateUi(base, NOW)).toEqual({
      lines: [],
      install: false,
      download: false,
      busy: false,
    });
  });

  it("says when the last check was", () => {
    const ui = updateUi({ ...base, status: { kind: "up_to_date" }, checked_at: NOW - 60_000 }, NOW);
    expect(ui.lines).toEqual([
      { key: "about-update-current" },
      { key: "about-update-checked", args: { ms: 60_000 } },
    ]);
  });

  it("offers the install only to an installed copy", () => {
    const installed = updateUi({ ...base, status: { kind: "found" }, available: found }, NOW);
    expect(installed.install).toBe(true);
    expect(installed.download).toBe(false);
    expect(installed.lines[0]).toEqual({
      key: "about-update-available",
      args: { version: "0.1.1" },
    });

    const portable = updateUi(
      { ...base, status: { kind: "found" }, available: found, can_install: false },
      NOW,
    );
    expect(portable.install).toBe(false);
    expect(portable.download).toBe(true);
    expect(portable.lines).toContainEqual({ key: "about-update-portable" });
  });

  it("waits while checking or installing", () => {
    const installing = updateUi(
      { ...base, status: { kind: "installing", percent: 40 }, available: found, checked_at: NOW },
      NOW,
    );
    expect(installing.busy).toBe(true);
    expect(installing.install).toBe(false);
    expect(installing.lines).toEqual([
      { key: "about-update-available", args: { version: "0.1.1" } },
      { key: "about-update-downloading-pct", args: { pct: 40 } },
    ]);
    expect(updateUi({ ...base, status: { kind: "checking" } }, NOW).busy).toBe(true);
  });

  it("keeps a found update after a failed install", () => {
    const ui = updateUi(
      { ...base, status: { kind: "failed", stage: "install" }, available: found },
      NOW,
    );
    expect(ui.install).toBe(true);
    expect(ui.lines).toContainEqual({ key: "about-update-install-failed" });
  });
});

describe("statusline items", () => {
  const items = (...k: SegmentKind[]) => k.map((kind) => newItem(kind));
  const kinds = (list: Item[]) => list.map((i) => (i.enabled ? i.kind : `(${i.kind})`));

  it("keeps the last value item visible", () => {
    const list = items("cost", "line_break");
    expect(canDrop(list, 0)).toBe(false);
    expect(canDrop(list, 1)).toBe(true);
    expect(toggleHidden(list, 0)).toEqual(list);
    expect(removeItem(list, 0)).toEqual(list);
    expect(kinds(removeItem(list, 1))).toEqual(["cost"]);
    const two = items("cost", "model");
    const hidden = toggleHidden(two, 0);
    expect(kinds(hidden)).toEqual(["(cost)", "model"]);
    expect(canDrop(hidden, 1)).toBe(false);
    expect(canDrop(hidden, 0)).toBe(true);
    expect(kinds(toggleHidden(hidden, 0))).toEqual(["cost", "model"]);
  });

  it("moves an item to an index and maps drop slots to it", () => {
    const list = items("five_hour", "seven_day", "cost", "model");
    expect(kinds(moveItem(list, 0, 2))).toEqual(["seven_day", "cost", "five_hour", "model"]);
    expect(kinds(moveItem(list, 3, 0))).toEqual(["model", "five_hour", "seven_day", "cost"]);
    expect(moveItem(list, 0, 4)).toEqual(list);
    expect(moveItem(list, -1, 0)).toEqual(list);
    // Dropping row 0 into the gap before row 2 puts it at index 1.
    expect(dropIndex(0, 2)).toBe(1);
    expect(dropIndex(0, 4)).toBe(3);
    expect(dropIndex(3, 0)).toBe(0);
    expect(dropIndex(2, 2)).toBe(2);
  });

  it("adds layout items freely and value items once, after the selection", () => {
    const list = items("five_hour", "cost");
    expect(canAdd(list, "cost")).toBe(false);
    expect(canAdd(list, "separator")).toBe(true);
    expect(addItem(list, "cost", null)).toBeNull();
    const added = addItem(items("five_hour", "separator", "cost"), "model", 1);
    expect(added?.index).toBe(2);
    // The separator before it is there already; one goes between it and `cost`.
    expect(kinds(added?.items ?? [])).toEqual([
      "five_hour",
      "separator",
      "model",
      "separator",
      "cost",
    ]);
    expect(addItem(list, "separator", null)?.index).toBe(2);
    const full = Array.from({ length: MAX_ITEMS }, () => newItem("separator"));
    expect(canAdd(full, "separator")).toBe(false);
  });

  it("gives a new separator the toolbar's glyph and counts space and flex as layout", () => {
    const list = items("cost");
    expect(addItem(list, "separator", 0, "›")?.items[1]?.text).toBe("›");
    // Blank: the core's default glyph.
    expect(addItem(list, "separator", 0, " ")?.items[1]?.text).toBe("");
    expect(newItem("custom_text", "›").text).toBe("");
    expect(canAdd([...list, newItem("flex")], "flex")).toBe(true);
    expect(canDrop([newItem("space"), ...list], 1)).toBe(false);
  });

  it("patches one item and picks the next selection after a removal", () => {
    const list = items("five_hour", "cost");
    expect(updateItem(list, 1, { label: "$" })[1]?.label).toBe("$");
    expect(updateItem(list, 1, { label: "$" })[0]).toBe(list[0]);
    expect(selectionAfterRemove(3, 2)).toBe(1);
    expect(selectionAfterRemove(3, 0)).toBe(0);
    expect(selectionAfterRemove(1, 0)).toBeNull();
    expect(kindKey("session_share")).toBe("statusline-kind-session-share");
  });

  it("groups the palette and marks used and absent kinds", () => {
    const kind = (
      k: SegmentKind,
      group: StatuslineKind["group"],
      absent = false,
    ): StatuslineKind => ({
      kind: k,
      group,
      formats: [],
      default_format: null,
      labeled: false,
      default_label: "",
      colored: false,
      sample: k,
      absent,
      reset_formats: [],
      bar_styles: [],
    });
    const catalog = [
      kind("five_hour", "limit", true),
      kind("seven_day", "limit"),
      kind("cwd", "project"),
      kind("separator", "layout"),
      kind("line_break", "layout"),
      kind("custom_text", "layout"),
      kind("flex", "layout"),
    ];
    const groups = palette(catalog, items("seven_day", "separator"));
    expect(groups.map((g) => g.group)).toEqual(["limit", "project", "layout"]);
    // No line break ("Add line" makes one); the layout group in its own order.
    expect(groups[2]?.entries.map((e) => e.kind)).toEqual(["separator", "flex", "custom_text"]);
    expect(groups[0]?.entries).toEqual([
      { kind: "five_hour", sample: "five_hour", used: false, absent: true },
      { kind: "seven_day", sample: "seven_day", used: true, absent: false },
    ]);
    expect(groups[2]?.entries[0]?.used).toBe(false);
  });
});

describe("statusline lines", () => {
  const items = (...k: SegmentKind[]) => k.map((kind) => newItem(kind));
  const kinds = (list: Item[]) => list.map((i) => (i.enabled ? i.kind : `(${i.kind})`));
  const hidden = (item: Item): Item => ({ ...item, enabled: false });

  it("splits the list into cards at the line breaks the core draws", () => {
    expect(lines([])).toEqual([{ items: [], end: null }]);
    const list = items("five_hour", "line_break", "cost", "line_break", "line_break", "model");
    expect(lines(list)).toEqual([
      { items: [0], end: 1 },
      { items: [2], end: 3 },
      { items: [], end: 4 },
      { items: [5], end: null },
    ]);
    // A hidden line break does not split; it is a row of its card.
    const off = [list[0] as Item, hidden(list[1] as Item), list[2] as Item];
    expect(lines(off)).toEqual([{ items: [0, 1, 2], end: null }]);
    expect(placeOf(list, 5)).toEqual({ line: 3, position: 0, count: 1 });
    expect(placeOf(list, 1)).toBeNull();
  });

  it("adds a value to a card with separators between values", () => {
    const list = items("five_hour", "line_break", "cost");
    // Not selected in the card: at its end, after a separator.
    const end = addToLine(list, "model", 0, 2, "›");
    expect(kinds(end?.items ?? [])).toEqual([
      "five_hour",
      "separator",
      "model",
      "line_break",
      "cost",
    ]);
    expect(end?.index).toBe(2);
    expect(end?.items[1]?.text).toBe("›");
    // After the selection in the card.
    const after = addToLine(list, "model", 1, 2);
    expect(kinds(after?.items ?? [])).toEqual([
      "five_hour",
      "line_break",
      "cost",
      "separator",
      "model",
    ]);
    // Into an empty card, and a layout item never brings a separator.
    const empty = addToLine(items("cost", "line_break"), "model", 1, null);
    expect(kinds(empty?.items ?? [])).toEqual(["cost", "line_break", "model"]);
    expect(kinds(addToLine(list, "flex", 0, 0)?.items ?? [])).toEqual([
      "five_hour",
      "flex",
      "line_break",
      "cost",
    ]);
    // A hidden value next to it still gets one: the core drops it while the value is hidden.
    const off = [hidden(newItem("cost")), newItem("five_hour")];
    expect(kinds(addItem(off, "model", 0)?.items ?? [])).toEqual([
      "(cost)",
      "separator",
      "model",
      "separator",
      "five_hour",
    ]);
  });

  it("leaves separators out when the list is full", () => {
    const full = [
      newItem("cost"),
      ...Array.from({ length: MAX_ITEMS - 2 }, () => newItem("space")),
    ];
    const added = addItem(full, "model", 0);
    expect(added?.items.length).toBe(MAX_ITEMS);
    expect(kinds(added?.items ?? []).slice(0, 2)).toEqual(["cost", "model"]);
  });

  it("adds and removes cards, keeping the last value shown", () => {
    const list = items("five_hour", "line_break", "cost", "model");
    const added = addLine(list) ?? [];
    expect(lines(added).length).toBe(3);
    expect(addLine(Array.from({ length: MAX_ITEMS }, () => newItem("space")))).toBeNull();
    // The first card goes with the break that ends it.
    expect(removeLine(list, 0)).toEqual({ items: [list[2], list[3]], start: 0, count: 2 });
    // Another goes with the break before it.
    const second = removeLine(list, 1);
    expect(kinds(second?.items ?? [])).toEqual(["five_hour"]);
    expect(second?.start).toBe(1);
    expect(kinds(removeLine(added, 2)?.items ?? [])).toEqual(kinds(list));
    // The only card, or one that takes the last shown value, stays.
    expect(canRemoveLine(items("cost", "model"), 0)).toBe(false);
    const last = [hidden(newItem("five_hour")), newItem("line_break"), newItem("cost")];
    expect(canRemoveLine(last, 1)).toBe(false);
    expect(canRemoveLine(last, 0)).toBe(true);
    expect(removeLine(last, 1)).toBeNull();
  });

  it("moves an item between cards", () => {
    const list = items("five_hour", "seven_day", "line_break", "cost", "line_break");
    // To the end of the next card, the start of it and into the empty last one.
    expect(kinds(moveToLine(list, 0, 1, Number.POSITIVE_INFINITY)?.items ?? [])).toEqual([
      "seven_day",
      "line_break",
      "cost",
      "five_hour",
      "line_break",
    ]);
    expect(moveToLine(list, 0, 1, 0)?.index).toBe(2);
    const empty = moveToLine(list, 3, 2, 0);
    expect(kinds(empty?.items ?? [])).toEqual([
      "five_hour",
      "seven_day",
      "line_break",
      "line_break",
      "cost",
    ]);
    expect(empty?.index).toBe(4);
    expect(moveToLine(list, 2, 0, 0)).toBeNull();
    expect(moveToLine(list, 0, 5, 0)).toBeNull();
    // A drop counts the gap with the item in place in its own card.
    expect(kinds(dropInto(list, 0, 0, 2)?.items ?? []).slice(0, 2)).toEqual([
      "seven_day",
      "five_hour",
    ]);
    expect(dropInto(list, 0, 0, 1)).toBeNull();
    expect(dropInto(list, 3, 0, 0)?.index).toBe(0);
    // Alt+arrows: a swap with the line break moves the item into the next card.
    expect(placeOf(moveItem(list, 1, 2), 2)).toEqual({ line: 1, position: 0, count: 2 });
  });

  it("words what a flexible space does by the card's count", () => {
    const one = items("cost", "flex", "model", "line_break", "flex", "five_hour", "flex");
    expect(flexHintKey(one, 1)).toBe("statusline-flex-right");
    expect(flexHintKey(one, 4)).toBe("statusline-flex-center");
    const three = items("flex", "cost", "flex", "model", "flex");
    expect(flexHintKey(three, 0)).toBe("statusline-flex-spread");
    expect(flexHintKey([hidden(newItem("flex")), ...three.slice(1, 3)], 2)).toBe(
      "statusline-flex-right",
    );
  });

  it("applies a glyph to every separator and to new ones", () => {
    const prefs: StatuslinePrefs = {
      version: 2,
      items: items("cost", "separator", "model", "separator", "custom_text"),
      theme: "plain",
      separator: "│",
      color: "auto",
      margin: 0,
      icons: "text",
    };
    const next = applySeparator(prefs, "·");
    expect(next.separator).toBe("·");
    expect(next.items.map((i) => i.text)).toEqual(["", "·", "", "·", ""]);
  });
});

describe("statusline preview", () => {
  const span = (text: string, item: number | null, more: Partial<Span> = {}): Span => ({
    text,
    paint: "plain",
    fg: null,
    bg: null,
    item,
    ...more,
  });

  it("finds an item's text across lines", () => {
    const preview = {
      lines: [[span("7d ", 0), span("42%", 0), span(" │ ", null)], [span("API $4.12", 2)]],
      absent: [],
    };
    expect(itemText(preview, 0)).toBe("7d 42%");
    expect(itemText(preview, 2)).toBe("API $4.12");
    expect(itemText(preview, 1)).toBeNull();
  });

  it("lets the user's color replace the paint and keeps faint on top", () => {
    expect(spanClasses(span("a", 0))).toBe("");
    expect(spanClasses(span("a", 0, { paint: "warn" }))).toBe("p-warn");
    expect(spanClasses(span("a", 0, { paint: "ok", fg: "bright_cyan" }))).toBe("fg-bright-cyan");
    expect(spanClasses(span("a", 0, { paint: "dim", fg: "gray", bg: "blue" }))).toBe(
      "fg-gray p-dim bg-blue",
    );
  });
});

describe("statusline icons and bar styles", () => {
  const font = (tier: "none" | "old" | "full", mono = true) => ({ family: "F", tier, mono });

  it("warns about the font only when it is known and short of what is asked", () => {
    expect(fontNoteKey("icons", null, false)).toBeNull();
    expect(fontNoteKey("icons", null, true)).toBe("statusline-icons-no-font");
    expect(fontNoteKey("icons", font("old"), true)).toBeNull();
    expect(fontNoteKey("icons", font("full", false), true)).toBe("statusline-icons-not-mono");
    expect(fontNoteKey("capsule", null, true)).toBe("statusline-capsule-no-font");
    expect(fontNoteKey("capsule", font("old"), true)).toBe("statusline-capsule-old-font");
    expect(fontNoteKey("capsule", font("full", false), true)).toBeNull();
  });

  it("colors only the solid bar's runs in the style picker", () => {
    const cell = (text: string, more: Partial<Span> = {}): Span => ({
      text,
      paint: "ok",
      fg: null,
      bg: null,
      item: null,
      ...more,
    });
    expect(glyphRuns([cell("██")])).toEqual([{ text: "██", color: null, background: null }]);
    expect(glyphRuns([cell("  ", { bg: "cyan" }), cell("|", { fg: "black", bg: "gray" })])).toEqual(
      [
        { text: "  ", color: null, background: "var(--term-cyan)" },
        { text: "|", color: "var(--term-black)", background: "var(--term-gray)" },
      ],
    );
  });
});

describe("statusline undo", () => {
  const prefs = (separator: string): StatuslinePrefs => ({
    version: 2,
    items: [newItem("cost")],
    theme: "plain",
    separator,
    color: "auto",
    margin: 0,
    icons: "text",
  });

  it("undoes step by step and joins quick edits of one field", () => {
    let h = emptyHistory();
    h = record(h, prefs("a"), null, 0);
    h = record(h, prefs("b"), "label:0", 100);
    h = record(h, prefs("c"), "label:0", 600);
    h = record(h, prefs("d"), "label:0", 1500);
    expect(h.past.map((p) => p.separator)).toEqual(["a", "b"]);
    h = record(h, prefs("e"), "label:0", 2600);
    expect(h.past.map((p) => p.separator)).toEqual(["a", "b", "e"]);
    const back = undo(h);
    expect(back?.prefs.separator).toBe("e");
    // After an undo the next edit starts a new step.
    expect(record(back?.history ?? h, prefs("f"), "label:0", 2700).past).toHaveLength(3);
    expect(undo(emptyHistory())).toBeNull();
  });

  it("keeps at most HISTORY_LIMIT steps", () => {
    let h = emptyHistory();
    for (let n = 0; n < HISTORY_LIMIT + 5; n++) h = record(h, prefs(String(n)), null, n);
    expect(h.past).toHaveLength(HISTORY_LIMIT);
    expect(h.past[0]?.separator).toBe("5");
  });
});

describe("statusline saver", () => {
  it("saves the last of rapid changes once, and one save at a time", async () => {
    vi.useFakeTimers();
    try {
      const saved: number[] = [];
      let release: () => void = () => {};
      const saver = createSaver<number>(400, async (v) => {
        saved.push(v);
        if (v === 2) await new Promise<void>((r) => (release = r));
      });
      saver.schedule(1);
      vi.advanceTimersByTime(200);
      saver.schedule(2);
      expect(saver.pending()).toBe(true);
      vi.advanceTimersByTime(400);
      expect(saved).toEqual([2]);
      // A change while a save runs waits for it.
      saver.schedule(3);
      vi.advanceTimersByTime(400);
      expect(saved).toEqual([2]);
      release();
      await vi.runAllTimersAsync();
      expect(saved).toEqual([2, 3]);
      expect(saver.pending()).toBe(false);
    } finally {
      vi.useRealTimers();
    }
  });

  it("flushes a waiting value at once", async () => {
    const saved: number[] = [];
    const saver = createSaver<number>(10_000, async (v) => {
      saved.push(v);
    });
    saver.schedule(7);
    await saver.flush();
    expect(saved).toEqual([7]);
    expect(saver.pending()).toBe(false);
    await saver.flush();
    expect(saved).toEqual([7]);
  });
});

describe("statusline message keys", () => {
  // Every kind and format the core has (prefs.rs `SegmentKind::ALL`, `Format::ALL`).
  const KINDS: SegmentKind[] = [
    "five_hour",
    "seven_day",
    "scoped",
    "context",
    "session_share",
    "cost",
    "model",
    "tokens_in",
    "tokens_out",
    "duration",
    "api_duration",
    "lines_changed",
    "vim_mode",
    "version",
    "agent_name",
    "session_id",
    "cwd",
    "project_dir",
    "git_branch",
    "daily_budget",
    "burn_rate",
    "session_name",
    "cache_ratio",
    "output_style",
    "compact",
    "separator",
    "line_break",
    "custom_text",
    "space",
    "flex",
  ];
  const FORMATS = [
    "bar",
    "percent",
    "fraction",
    "two_decimals",
    "four_decimals",
    "short",
    "full",
    "compact",
    "hms",
    "both",
    "net",
    "basename",
    "home",
    "remaining",
    "tokens_left",
    "total",
    "split",
  ] as const;
  const PRESETS = ["default", "minimal", "detailed", "two-line", "powerline", "split"];

  it("has a message for every key the editor builds", () => {
    const ftl = readFileSync(
      resolve(import.meta.dirname, "../../../../locales/en/settings.ftl"),
      "utf8",
    );
    const keys = [
      ...KINDS.flatMap((k) => [kindKey(k), kindHintKey(k)]),
      ...FORMATS.map(formatKey),
      ...ANSI_COLORS.map(colorKey),
      ...GROUPS.map(groupKey),
      ...THEMES.map(themeKey),
      ...COLOR_MODES.map(colorModeKey),
      ...SAMPLE_STATES.map(stateKey),
      ...(["countdown", "clock"] as const).map(resetFormatKey),
      ...(["blocks", "thin", "dots", "capsule", "solid"] as const).map(barStyleKey),
      ...ICON_SETS.map(iconSetKey),
      ...(["icons", "capsule"] as const)
        .flatMap((need) =>
          [null, { family: "X", tier: "old" as const, mono: false }].map((font) =>
            fontNoteKey(need, font, true),
          ),
        )
        .filter((k): k is string => k !== null),
      "statusline-icons",
      "statusline-icons-suggest",
      "statusline-icons-use-nerd",
      "statusline-bar-in-blocks",
      "statusline-reset-format",
      "statusline-bar-width",
      "statusline-bar-style",
      ...PRESETS.flatMap((p) => [`statusline-preset-${p}`, `statusline-preset-${p}-hint`]),
      ...[1, 2, 3].map((n) =>
        flexHintKey(
          Array.from({ length: n }, () => newItem("flex")),
          0,
        ),
      ),
    ];
    const missing = keys.filter((k) => !new RegExp(`^${k} =`, "m").test(ftl));
    expect(missing).toEqual([]);
  });
});

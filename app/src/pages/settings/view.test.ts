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
  addThreshold,
  apiStatus,
  COLOR_MODES,
  canAdd,
  canDrop,
  colorKey,
  colorModeKey,
  consentAge,
  createSaver,
  dropIndex,
  emptyHistory,
  formatKey,
  GROUPS,
  groupKey,
  HISTORY_LIMIT,
  itemText,
  kindHintKey,
  kindKey,
  MAX_ITEMS,
  MAX_THRESHOLDS,
  moveItem,
  newItem,
  optionsWith,
  palette,
  parseThreshold,
  record,
  removeItem,
  removeThreshold,
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
  const items = (...k: SegmentKind[]) => k.map(newItem);
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
    const added = addItem(list, "model", 0);
    expect(added?.index).toBe(1);
    expect(kinds(added?.items ?? [])).toEqual(["five_hour", "model", "cost"]);
    expect(addItem(list, "separator", null)?.index).toBe(2);
    const full = Array.from({ length: MAX_ITEMS }, () => newItem("separator"));
    expect(canAdd(full, "separator")).toBe(false);
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
    });
    const catalog = [
      kind("five_hour", "limit", true),
      kind("seven_day", "limit"),
      kind("cwd", "project"),
      kind("separator", "layout"),
    ];
    const groups = palette(catalog, items("seven_day", "separator"));
    expect(groups.map((g) => g.group)).toEqual(["limit", "project", "layout"]);
    expect(groups[0]?.entries).toEqual([
      { kind: "five_hour", sample: "five_hour", used: false, absent: true },
      { kind: "seven_day", sample: "seven_day", used: true, absent: false },
    ]);
    expect(groups[2]?.entries[0]?.used).toBe(false);
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

describe("statusline undo", () => {
  const prefs = (separator: string): StatuslinePrefs => ({
    items: [newItem("cost")],
    theme: "plain",
    separator,
    color: "auto",
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
    "separator",
    "line_break",
    "custom_text",
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
  ] as const;
  const PRESETS = ["default", "minimal", "detailed", "two-line", "powerline"];

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
      ...PRESETS.flatMap((p) => [`statusline-preset-${p}`, `statusline-preset-${p}-hint`]),
    ];
    const missing = keys.filter((k) => !new RegExp(`^${k} =`, "m").test(ftl));
    expect(missing).toEqual([]);
  });
});

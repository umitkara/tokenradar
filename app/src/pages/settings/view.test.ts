import { describe, expect, it } from "vitest";
import type { ApiConsentView, UpdateView } from "../../lib/ipc/bindings";
import {
  addThreshold,
  apiStatus,
  consentAge,
  MAX_THRESHOLDS,
  optionsWith,
  parseThreshold,
  removeThreshold,
  sizeMessage,
  targetFor,
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

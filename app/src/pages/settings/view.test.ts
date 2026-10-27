import { describe, expect, it } from "vitest";
import type { ApiConsentView } from "../../lib/ipc/bindings";
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
} from "./view";

describe("targetFor", () => {
  it("maps panel sections to a tab and an anchor", () => {
    expect(targetFor("api")).toEqual({ tab: "sources", anchor: "api" });
    expect(targetFor("statusline")).toEqual({ tab: "sources", anchor: "statusline" });
    expect(targetFor("accounts")).toEqual({ tab: "sources", anchor: "dirs" });
    expect(targetFor("diagnostics")).toEqual({ tab: "privacy", anchor: "diagnostics" });
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

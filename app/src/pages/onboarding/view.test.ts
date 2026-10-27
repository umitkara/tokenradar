import { describe, expect, it } from "vitest";
import type { DataSourcesView, IndexState } from "../../lib/ipc/bindings";
import { indexView, limitsOutcome, pinView } from "./view";

function index(overrides: Partial<IndexState>): IndexState {
  return {
    initial_scan_done: false,
    last_indexed_at: null,
    progress: null,
    recent_ready: false,
    drift: null,
    ...overrides,
  };
}

function sources(overrides: Partial<DataSourcesView>): DataSourcesView {
  return {
    dirs: [],
    chosen_missing: null,
    env_dir: null,
    statusline: { state: "not_installed", current_command: null },
    statusline_last_seen: null,
    cli_missing: false,
    api: { granted: false, granted_at: null, outdated: false, status: null },
    ...overrides,
  };
}

describe("indexView", () => {
  it("waits until the core reports", () => {
    expect(indexView(null)).toEqual({ pct: null, lines: [{ key: "onb-claude-waiting" }] });
    expect(indexView(index({}))).toEqual({ pct: null, lines: [{ key: "onb-claude-waiting" }] });
  });

  it("shows progress by bytes and the recent milestone", () => {
    const view = indexView(
      index({
        progress: {
          files_done: 90,
          files_total: 100,
          bytes_done: 425,
          bytes_total: 1000,
          phase: "indexing",
        },
        recent_ready: true,
      }),
    );
    expect(view.pct).toBe(42);
    expect(view.lines).toEqual([
      { key: "onb-claude-indexing", args: { pct: 42 } },
      { key: "onb-claude-recent-ready" },
    ]);
  });

  it("is full once the first scan finished", () => {
    expect(indexView(index({ initial_scan_done: true })).pct).toBe(100);
  });
});

describe("limitsOutcome", () => {
  it("prefers the statusline, then the API, else says what is missing", () => {
    expect(limitsOutcome(null)).toBeNull();
    expect(limitsOutcome(sources({}))).toBe("none");
    expect(
      limitsOutcome(
        sources({ api: { granted: true, granted_at: null, outdated: false, status: null } }),
      ),
    ).toBe("api");
    expect(
      limitsOutcome(
        sources({
          statusline: { state: "installed", chained_command: null },
          api: { granted: true, granted_at: null, outdated: false, status: null },
        }),
      ),
    ).toBe("statusline");
    expect(limitsOutcome(sources({ statusline: { state: "installed_elsewhere" } }))).toBe(
      "statusline",
    );
  });
});

describe("pinView", () => {
  it("offers the button until it was tried", () => {
    expect(pinView("hidden", "win11", false)).toEqual({
      status: null,
      offerPin: true,
      manual: null,
    });
    expect(pinView("unknown", "win11", false).offerPin).toBe(true);
  });

  it("falls back to the settings page when the pin did not take", () => {
    expect(pinView("hidden", "win11", true)).toEqual({
      status: { key: "onb-tray-not-done" },
      offerPin: false,
      manual: { key: "onb-tray-manual" },
    });
  });

  it("says when the icon is already shown", () => {
    expect(pinView("shown", "win11", true).status).toEqual({ key: "onb-tray-shown" });
  });

  it("gives Windows 10 its own instructions", () => {
    expect(pinView("unsupported", "win10", false).manual).toEqual({
      key: "onb-tray-manual-win10",
    });
  });

  it("shows nothing before the state is known", () => {
    expect(pinView(null, "win11", false)).toEqual({ status: null, offerPin: false, manual: null });
  });
});

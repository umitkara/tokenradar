import { describe, expect, it } from "vitest";
import type { ProjectNode, ProjectWeek } from "../../../lib/ipc/bindings";
import { parentIndex, visibleNodes, weekBars } from "./view";

function node(depth: number, path: string, children = 0): ProjectNode {
  return {
    depth,
    label: path,
    path,
    project: null,
    projects: 1,
    sessions: 1,
    last_active: null,
    cost: 0,
    weekly_percent: null,
    trend: [],
    children,
  };
}

// a
// ├ a/b
// │ ├ a/b/c
// │ └ a/b/d
// └ a/e
// f
const TREE = [
  node(0, "a", 2),
  node(1, "a/b", 2),
  node(2, "a/b/c"),
  node(2, "a/b/d"),
  node(1, "a/e"),
  node(0, "f"),
];
const paths = (nodes: ProjectNode[]) => nodes.map((n) => n.path);

describe("visibleNodes", () => {
  it("shows everything when nothing is collapsed", () => {
    expect(visibleNodes(TREE, new Set())).toHaveLength(TREE.length);
  });

  it("hides the subtree of a collapsed folder only", () => {
    expect(paths(visibleNodes(TREE, new Set(["a/b"])))).toEqual(["a", "a/b", "a/e", "f"]);
    expect(paths(visibleNodes(TREE, new Set(["a"])))).toEqual(["a", "f"]);
  });

  it("ignores a collapsed leaf", () => {
    expect(visibleNodes(TREE, new Set(["a/e"]))).toHaveLength(TREE.length);
  });
});

describe("parentIndex", () => {
  it("finds the nearest shallower node before", () => {
    expect(parentIndex(TREE, 3)).toBe(1);
    expect(parentIndex(TREE, 4)).toBe(0);
    expect(parentIndex(TREE, 0)).toBeNull();
    expect(parentIndex(TREE, 5)).toBeNull();
  });
});

describe("weekBars", () => {
  const week = (id: number, percent: number | null, cost: number): ProjectWeek => ({
    window_id: id,
    starts_at: id * 1000,
    resets_at: id * 1000 + 999,
    estimated: false,
    percent,
    cost,
  });

  it("scales shares to the largest", () => {
    const { bars, byCost } = weekBars([week(1, 5, 900), week(2, 10, 1), week(3, null, 50)]);
    expect(byCost).toBe(false);
    expect(bars.map((b) => b.height)).toEqual([50, 100, 0]);
  });

  it("falls back to cost when no week has a share", () => {
    const { bars, byCost } = weekBars([week(1, null, 200), week(2, null, 800)]);
    expect(byCost).toBe(true);
    expect(bars.map((b) => b.height)).toEqual([25, 100]);
  });

  it("draws nothing for all-zero weeks", () => {
    expect(weekBars([week(1, 0, 0)]).bars[0]?.height).toBe(0);
    expect(weekBars([]).bars).toEqual([]);
  });
});

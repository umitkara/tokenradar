import { describe, expect, it } from "vitest";
import { isNavKey, moveSelection, scrollToShow, visibleRange } from "./virtual";

describe("visibleRange", () => {
  it("renders the viewport plus overscan, clamped", () => {
    expect(visibleRange(0, 400, 40, 10_000, 5)).toEqual({ start: 0, end: 16 });
    expect(visibleRange(4000, 400, 40, 10_000, 5)).toEqual({ start: 95, end: 116 });
    expect(visibleRange(399_600, 400, 40, 10_000, 5)).toEqual({ start: 9985, end: 10_000 });
  });

  it("keeps a slice of constant size however long the list is", () => {
    const small = visibleRange(40_000, 600, 40, 2_000);
    const big = visibleRange(40_000, 600, 40, 1_000_000);
    expect(small.end - small.start).toBe(big.end - big.start);
  });

  it("handles empty lists, negative scroll and a scroll past the end", () => {
    expect(visibleRange(0, 400, 40, 0)).toEqual({ start: 0, end: 0 });
    expect(visibleRange(-50, 400, 40, 3, 2)).toEqual({ start: 0, end: 3 });
    expect(visibleRange(1e9, 400, 40, 3, 2)).toEqual({ start: 2, end: 3 });
  });
});

describe("moveSelection", () => {
  it("steps, pages and jumps within bounds", () => {
    expect(moveSelection("ArrowDown", 0, 10, 5)).toBe(1);
    expect(moveSelection("ArrowUp", 0, 10, 5)).toBe(0);
    expect(moveSelection("ArrowDown", 9, 10, 5)).toBe(9);
    expect(moveSelection("PageDown", 2, 10, 5)).toBe(6);
    expect(moveSelection("PageUp", 2, 10, 5)).toBe(0);
    expect(moveSelection("Home", 7, 10, 5)).toBe(0);
    expect(moveSelection("End", 1, 10, 5)).toBe(9);
  });

  it("selects the first row (the last for End) when nothing is selected", () => {
    expect(moveSelection("ArrowUp", null, 10, 5)).toBe(0);
    expect(moveSelection("End", null, 10, 5)).toBe(9);
    expect(moveSelection("ArrowDown", null, 0, 5)).toBeNull();
  });

  it("knows its keys", () => {
    expect(isNavKey("PageDown")).toBe(true);
    expect(isNavKey("Enter")).toBe(false);
  });
});

describe("scrollToShow", () => {
  it("moves only when the row is out of view, as little as possible", () => {
    expect(scrollToShow(5, 0, 400, 40)).toBeNull();
    expect(scrollToShow(12, 0, 400, 40)).toBe(120);
    expect(scrollToShow(2, 200, 400, 40)).toBe(80);
  });
});

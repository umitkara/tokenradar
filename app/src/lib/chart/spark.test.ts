import { describe, expect, it } from "vitest";
import { spanBox } from "./model";
import { sparkPoints } from "./spark";

describe("sparkPoints", () => {
  it("spreads the values across the box with room for the stroke", () => {
    expect(sparkPoints([0, 5, 10], 22, 12)).toBe("1,11 11,6 21,1");
  });

  it("lays a flat, single or empty series on the baseline", () => {
    expect(sparkPoints([0, 0], 10, 10)).toBe("1,9 9,9");
    expect(sparkPoints([3], 10, 10)).toBe("1,1");
    expect(sparkPoints([], 10, 10)).toBe("");
  });
});

describe("spanBox", () => {
  it("places a span on the plot's time scale", () => {
    expect(spanBox({ from: 25, to: 50 }, 0, 100, 400)).toEqual({ left: 100, width: 100 });
  });

  it("clips to the range and keeps short spans visible", () => {
    expect(spanBox({ from: -50, to: 10 }, 0, 100, 400)).toEqual({ left: 0, width: 40 });
    expect(spanBox({ from: 50, to: 50 }, 0, 100, 400)).toEqual({ left: 200, width: 6 });
    expect(spanBox({ from: 100, to: 100 }, 0, 100, 400)).toEqual({ left: 394, width: 6 });
  });

  it("drops spans outside the range", () => {
    expect(spanBox({ from: 120, to: 130 }, 0, 100, 400)).toBeNull();
    expect(spanBox({ from: 0, to: 10 }, 100, 100, 400)).toBeNull();
  });
});

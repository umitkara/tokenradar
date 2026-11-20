import { describe, expect, it } from "vitest";
import { MAX_PLAN_FEE_USD, parseFee } from "./view";

describe("parseFee", () => {
  it("reads either decimal point and a leading dollar sign", () => {
    expect(parseFee("120")).toBe(120);
    expect(parseFee(" 99,5 ")).toBe(99.5);
    expect(parseFee("$200.00")).toBe(200);
  });

  it("treats empty as back to the plan table", () => {
    expect(parseFee("")).toBeNull();
    expect(parseFee("   ")).toBeNull();
  });

  it("rejects zero, negatives, words and typos", () => {
    for (const bad of ["0", "-5", "abc", "1.234,5", "12.345", String(MAX_PLAN_FEE_USD + 1)]) {
      expect(parseFee(bad)).toBeUndefined();
    }
  });
});

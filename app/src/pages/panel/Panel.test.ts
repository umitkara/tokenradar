import { render } from "svelte/server";
import { describe, expect, it } from "vitest";
import Panel from "./Panel.svelte";

describe("Panel", () => {
  it("renders the product name", () => {
    const { body } = render(Panel);
    expect(body).toContain("TokenRadar");
  });
});

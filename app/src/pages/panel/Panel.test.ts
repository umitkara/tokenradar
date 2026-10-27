import { render } from "svelte/server";
import { describe, expect, it } from "vitest";
import { panelCatalog } from "../../lib/i18n/catalogs";
import { initI18n } from "../../lib/i18n/i18n.svelte";
import Panel from "./Panel.svelte";

describe("Panel", () => {
  it("renders the product name from the catalog", () => {
    initI18n(panelCatalog, "tr");
    const { body } = render(Panel);
    expect(body).toContain("TokenRadar");
    expect(body).toContain("Yükleniyor");
  });
});

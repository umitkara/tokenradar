import { render } from "svelte/server";
import { describe, expect, it } from "vitest";
import { settingsCatalog } from "../../lib/i18n/catalogs";
import { initI18n } from "../../lib/i18n/i18n.svelte";
import Settings from "./Settings.svelte";

describe("Settings", () => {
  it("renders the five tabs in Turkish", () => {
    initI18n(settingsCatalog, "tr");
    const { body } = render(Settings);
    for (const tab of ["Genel", "Veri kaynakları", "Bildirimler", "Gizlilik ve veri", "Hakkında"]) {
      expect(body).toContain(tab);
    }
    expect(body).toContain('aria-selected="true"');
  });
});

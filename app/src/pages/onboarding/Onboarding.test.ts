import { render } from "svelte/server";
import { describe, expect, it } from "vitest";
import { initI18n } from "../../lib/i18n/i18n.svelte";
import { onboardingCatalog } from "./catalog";
import Onboarding from "./Onboarding.svelte";

describe("Onboarding", () => {
  it("opens on the welcome step in Turkish", () => {
    initI18n(onboardingCatalog, "tr");
    const { body } = render(Onboarding);
    expect(body).toContain("TokenRadar'a hoş geldiniz");
    expect(body).toContain("Adım 1 / 4");
    expect(body).toContain("İleri");
    expect(body).not.toContain("Geri");
  });

  it("opens on the welcome step in English", () => {
    initI18n(onboardingCatalog, "en");
    const { body } = render(Onboarding);
    expect(body).toContain("Welcome to TokenRadar");
    expect(body).toContain("Step 1 of 4");
  });
});

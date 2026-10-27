// Onboarding's messages; it reuses the settings' statusline, consent and folder texts.

import commonEn from "../../../../locales/en/common.ftl?raw";
import onboardingEn from "../../../../locales/en/onboarding.ftl?raw";
import settingsEn from "../../../../locales/en/settings.ftl?raw";
import commonTr from "../../../../locales/tr/common.ftl?raw";
import onboardingTr from "../../../../locales/tr/onboarding.ftl?raw";
import settingsTr from "../../../../locales/tr/settings.ftl?raw";
import type { Catalog } from "../../lib/i18n/i18n.svelte";

export const onboardingCatalog: Catalog = {
  en: [commonEn, settingsEn, onboardingEn],
  tr: [commonTr, settingsTr, onboardingTr],
};

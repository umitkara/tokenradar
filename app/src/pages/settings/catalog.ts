// The settings window's messages (see pages/panel/catalog.ts).

import commonEn from "../../../../locales/en/common.ftl?raw";
import settingsEn from "../../../../locales/en/settings.ftl?raw";
import commonTr from "../../../../locales/tr/common.ftl?raw";
import settingsTr from "../../../../locales/tr/settings.ftl?raw";
import type { Catalog } from "../../lib/i18n/i18n.svelte";

export const settingsCatalog: Catalog = { en: [commonEn, settingsEn], tr: [commonTr, settingsTr] };

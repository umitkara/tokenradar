// The main window's messages (see pages/panel/catalog.ts). The panel's file is included: the
// overview's "Now" section is the panel's ring and rows, with the same projection and empty-state
// wording.

import commonEn from "../../../../locales/en/common.ftl?raw";
import mainEn from "../../../../locales/en/main.ftl?raw";
import panelEn from "../../../../locales/en/panel.ftl?raw";
import commonTr from "../../../../locales/tr/common.ftl?raw";
import mainTr from "../../../../locales/tr/main.ftl?raw";
import panelTr from "../../../../locales/tr/panel.ftl?raw";
import type { Catalog } from "../../lib/i18n/i18n.svelte";

export const mainCatalog: Catalog = {
  en: [commonEn, panelEn, mainEn],
  tr: [commonTr, panelTr, mainTr],
};

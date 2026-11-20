// The component gallery's messages: the panel's, the main window's (charts) and its own.

import commonEn from "../../../../locales/en/common.ftl?raw";
import galleryEn from "../../../../locales/en/gallery.ftl?raw";
import mainEn from "../../../../locales/en/main.ftl?raw";
import panelEn from "../../../../locales/en/panel.ftl?raw";
import commonTr from "../../../../locales/tr/common.ftl?raw";
import galleryTr from "../../../../locales/tr/gallery.ftl?raw";
import mainTr from "../../../../locales/tr/main.ftl?raw";
import panelTr from "../../../../locales/tr/panel.ftl?raw";
import type { Catalog } from "../../lib/i18n/i18n.svelte";

export const galleryCatalog: Catalog = {
  en: [commonEn, panelEn, mainEn, galleryEn],
  tr: [commonTr, panelTr, mainTr, galleryTr],
};

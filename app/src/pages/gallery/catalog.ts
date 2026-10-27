// The component gallery's messages: the panel's plus its own.

import commonEn from "../../../../locales/en/common.ftl?raw";
import galleryEn from "../../../../locales/en/gallery.ftl?raw";
import panelEn from "../../../../locales/en/panel.ftl?raw";
import commonTr from "../../../../locales/tr/common.ftl?raw";
import galleryTr from "../../../../locales/tr/gallery.ftl?raw";
import panelTr from "../../../../locales/tr/panel.ftl?raw";
import type { Catalog } from "../../lib/i18n/i18n.svelte";

export const galleryCatalog: Catalog = {
  en: [commonEn, panelEn, galleryEn],
  tr: [commonTr, panelTr, galleryTr],
};

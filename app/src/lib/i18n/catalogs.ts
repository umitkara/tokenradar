// Message catalogs per page. Each page imports only its own, so the panel bundle carries no
// settings or gallery text. The `.ftl` files are the same ones the Rust side embeds.

import commonEn from "../../../../locales/en/common.ftl?raw";
import galleryEn from "../../../../locales/en/gallery.ftl?raw";
import panelEn from "../../../../locales/en/panel.ftl?raw";
import settingsEn from "../../../../locales/en/settings.ftl?raw";
import commonTr from "../../../../locales/tr/common.ftl?raw";
import galleryTr from "../../../../locales/tr/gallery.ftl?raw";
import panelTr from "../../../../locales/tr/panel.ftl?raw";
import settingsTr from "../../../../locales/tr/settings.ftl?raw";
import type { Catalog } from "./i18n.svelte";

export const panelCatalog: Catalog = { en: [commonEn, panelEn], tr: [commonTr, panelTr] };
export const settingsCatalog: Catalog = {
  en: [commonEn, settingsEn],
  tr: [commonTr, settingsTr],
};
export const galleryCatalog: Catalog = {
  en: [commonEn, panelEn, galleryEn],
  tr: [commonTr, panelTr, galleryTr],
};

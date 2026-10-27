// The panel's messages. Each page has its own catalog module, so no page bundle carries
// another page's text. The `.ftl` files are the same ones the Rust side embeds.

import commonEn from "../../../../locales/en/common.ftl?raw";
import panelEn from "../../../../locales/en/panel.ftl?raw";
import commonTr from "../../../../locales/tr/common.ftl?raw";
import panelTr from "../../../../locales/tr/panel.ftl?raw";
import type { Catalog } from "../../lib/i18n/i18n.svelte";

export const panelCatalog: Catalog = { en: [commonEn, panelEn], tr: [commonTr, panelTr] };

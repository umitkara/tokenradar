// The main window's messages (see pages/panel/catalog.ts).

import commonEn from "../../../../locales/en/common.ftl?raw";
import mainEn from "../../../../locales/en/main.ftl?raw";
import commonTr from "../../../../locales/tr/common.ftl?raw";
import mainTr from "../../../../locales/tr/main.ftl?raw";
import type { Catalog } from "../../lib/i18n/i18n.svelte";

export const mainCatalog: Catalog = { en: [commonEn, mainEn], tr: [commonTr, mainTr] };

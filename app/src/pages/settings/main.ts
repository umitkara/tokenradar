import "../../lib/styles/tokens.css";
import "../../lib/styles/themes.css";
import "../../lib/styles/base.css";
import { mount } from "svelte";
import { settingsCatalog } from "../../lib/i18n/catalogs";
import { boot } from "../../lib/settings.svelte";
import Settings from "./Settings.svelte";

const target = document.getElementById("app");
if (!target) {
  throw new Error("Missing #app mount point");
}

boot(settingsCatalog);
export default mount(Settings, { target });

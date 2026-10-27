import "../../lib/styles/tokens.css";
import "../../lib/styles/themes.css";
import "../../lib/styles/base.css";
import { mount } from "svelte";
import { boot } from "../../lib/settings.svelte";
import { panelCatalog } from "./catalog";
import Panel from "./Panel.svelte";

const target = document.getElementById("app");
if (!target) {
  throw new Error("Missing #app mount point");
}

boot(panelCatalog);
export default mount(Panel, { target });

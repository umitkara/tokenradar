import "../../lib/styles/tokens.css";
import "../../lib/styles/themes.css";
import "../../lib/styles/base.css";
import { mount } from "svelte";
import { boot } from "../../lib/settings.svelte";
import { mainCatalog } from "./catalog";
import Main from "./Main.svelte";

const target = document.getElementById("app");
if (!target) {
  throw new Error("Missing #app mount point");
}

boot(mainCatalog);
export default mount(Main, { target });

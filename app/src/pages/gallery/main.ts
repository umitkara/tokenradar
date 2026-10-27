import "../../lib/styles/tokens.css";
import "../../lib/styles/themes.css";
import "../../lib/styles/base.css";
import { mount } from "svelte";
import { boot } from "../../lib/settings.svelte";
import { galleryCatalog } from "./catalog";
import Gallery from "./Gallery.svelte";

const target = document.getElementById("app");
if (!target) {
  throw new Error("Missing #app mount point");
}

boot(galleryCatalog);
export default mount(Gallery, { target });

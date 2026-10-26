import { mount } from "svelte";
import Panel from "./Panel.svelte";

const target = document.getElementById("app");
if (!target) {
  throw new Error("Missing #app mount point");
}

export default mount(Panel, { target });

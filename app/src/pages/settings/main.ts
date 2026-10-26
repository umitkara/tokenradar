import { mount } from "svelte";
import Settings from "./Settings.svelte";

const target = document.getElementById("app");
if (!target) {
  throw new Error("Missing #app mount point");
}

export default mount(Settings, { target });

import "../../lib/styles/tokens.css";
import "../../lib/styles/themes.css";
import "../../lib/styles/base.css";
import { mount } from "svelte";
import { boot } from "../../lib/settings.svelte";
import { onboardingCatalog } from "./catalog";
import Onboarding from "./Onboarding.svelte";

const target = document.getElementById("app");
if (!target) {
  throw new Error("Missing #app mount point");
}

boot(onboardingCatalog);
export default mount(Onboarding, { target });

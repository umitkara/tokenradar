<!--
  The first-run window (Step 11c): four steps with Back / Next. Nothing is applied by moving
  between steps; only the buttons inside a step act. "Finish" applies the autostart choice and
  closes the window, which records onboarding as done (so does closing it, ADR-035).
-->
<script lang="ts">
import { onMount, tick } from "svelte";
import Button from "../../lib/components/Button.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import { commands } from "../../lib/ipc/bindings";
import { inTauri } from "../../lib/ipc/runtime";
import { store } from "../settings/state.svelte";
import ClaudeStep from "./ClaudeStep.svelte";
import LimitsStep from "./LimitsStep.svelte";
import { startOnboarding } from "./state.svelte";
import TrayStep from "./TrayStep.svelte";
import { limitsOutcome, STEPS, TITLES } from "./view";
import WelcomeStep from "./WelcomeStep.svelte";

let current = $state(0);
let error = $state<string | null>(null);
/** Starting with Windows is on by default; the OS entry is written only on "Finish". */
let autostart = $state(true);
let finishing = $state(false);
let heading: HTMLElement;

const step = $derived(STEPS[current] ?? "welcome");
const last = $derived(current === STEPS.length - 1);
const skipping = $derived(step === "limits" && limitsOutcome(store.sources) === "none");

function onerror(key: string | null) {
  error = key;
}

async function go(next: number) {
  current = Math.max(0, Math.min(STEPS.length - 1, next));
  error = null;
  await tick();
  heading.focus();
}

async function finish() {
  if (!inTauri()) return;
  finishing = true;
  if (autostart !== store.autostart) {
    const r = await commands.setAutostart(autostart);
    if (r.status === "error") {
      finishing = false;
      error = "settings-save-failed";
      return;
    }
  }
  await commands.finishOnboarding();
}

onMount(() => {
  const stop = startOnboarding();
  if (inTauri()) {
    // "Finish" writes the OS entry only when the choice differs from it.
    void commands.getAutostart().then((r) => {
      store.autostart = r.status === "ok" ? r.data : null;
    });
  }
  return stop;
});
</script>

<div class="window">
  <header>
    <ol class="dots" aria-label={t("onboarding-steps")}>
      {#each STEPS as id, i (id)}
        <li class:done={i < current} class:current={i === current} aria-current={i === current ? "step" : undefined}>
          <span class="visually-hidden">{t(TITLES[id])}</span>
        </li>
      {/each}
    </ol>
    <p class="count">{t("onboarding-step", { n: current + 1, total: STEPS.length })}</p>
    <h1 bind:this={heading} tabindex="-1">{t(TITLES[step])}</h1>
  </header>

  <main>
    {#if error}<p class="error" role="alert">{t(error)}</p>{/if}
    {#if step === "welcome"}
      <WelcomeStep {onerror} />
    {:else if step === "claude"}
      <ClaudeStep {onerror} />
    {:else if step === "limits"}
      <LimitsStep {onerror} />
    {:else}
      <TrayStep bind:autostart {onerror} />
    {/if}
  </main>

  <footer>
    {#if current > 0}
      <Button onclick={() => go(current - 1)}>{t("onboarding-back")}</Button>
    {/if}
    <span class="spacer"></span>
    {#if last}
      <Button variant="primary" disabled={finishing} onclick={finish}>{t("onboarding-finish")}</Button>
    {:else}
      <Button variant={skipping ? "secondary" : "primary"} onclick={() => go(current + 1)}>
        {t(skipping ? "onboarding-skip" : "onboarding-next")}
      </Button>
    {/if}
  </footer>
</div>

<style>
  .window {
    display: grid;
    grid-template-rows: auto 1fr auto;
    height: 100vh;
  }
  header {
    padding: var(--space-5) var(--space-5) var(--space-3);
  }
  .dots {
    display: flex;
    gap: var(--space-2);
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .dots li {
    width: 24px;
    height: 4px;
    border-radius: var(--radius-pill);
    background: var(--track);
  }
  .dots li.done,
  .dots li.current {
    background: var(--accent);
  }
  .count {
    font: var(--type-caption);
    color: var(--text-2);
    margin-top: var(--space-3);
  }
  h1 {
    font: var(--type-display);
    margin-top: var(--space-1);
    outline: none;
  }
  main {
    overflow-y: auto;
    padding: var(--space-2) var(--space-5) var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  footer {
    display: flex;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-5);
    border-top: 1px solid var(--stroke);
  }
  .spacer {
    flex: 1;
  }
  .error {
    padding: var(--space-2) var(--space-3);
    border-left: 2px solid var(--signal-crit);
    background: var(--surface-1);
    border-radius: var(--radius-control);
  }
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  @media (forced-colors: active) {
    .dots li {
      background: GrayText;
    }
    .dots li.done,
    .dots li.current {
      background: Highlight;
    }
  }
</style>

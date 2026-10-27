<!--
  The settings window (Step 11b): a vertical tab list and the active tab.

  A panel link names a section (`settings.html#api` for a new window, `settings-section-requested`
  for an open one); `targetFor` turns it into a tab and the element to scroll to and focus.
-->
<script lang="ts">
import { onMount, tick } from "svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import { events } from "../../lib/ipc/bindings";
import { inTauri } from "../../lib/ipc/runtime";
import AboutTab from "./AboutTab.svelte";
import GeneralTab from "./GeneralTab.svelte";
import NotificationsTab from "./NotificationsTab.svelte";
import PrivacyTab from "./PrivacyTab.svelte";
import SourcesTab from "./SourcesTab.svelte";
import { startSettings } from "./state.svelte";
import { TABS, type Tab, targetFor } from "./view";

const LABELS: Record<Tab, string> = {
  general: "tab-general",
  sources: "tab-sources",
  notifications: "tab-notifications",
  privacy: "tab-privacy",
  about: "tab-about",
};

const initial = targetFor(typeof location === "undefined" ? null : location.hash.slice(1));
let tab = $state<Tab>(initial.tab);
let error = $state<string | null>(null);
let content: HTMLElement;

function onerror(key: string | null) {
  error = key;
}

async function show(next: Tab, anchor: string | null = null) {
  tab = next;
  error = null;
  await tick();
  content.scrollTop = 0;
  if (anchor) {
    const target = document.getElementById(anchor);
    target?.scrollIntoView({ block: "start" });
    target?.focus({ preventScroll: true });
  }
}

function keydown(e: KeyboardEvent) {
  const i = TABS.indexOf(tab);
  const next =
    e.key === "ArrowDown"
      ? TABS[(i + 1) % TABS.length]
      : e.key === "ArrowUp"
        ? TABS[(i + TABS.length - 1) % TABS.length]
        : e.key === "Home"
          ? TABS[0]
          : e.key === "End"
            ? TABS[TABS.length - 1]
            : null;
  if (!next) return;
  e.preventDefault();
  void show(next);
  document.getElementById(`tab-${next}`)?.focus();
}

onMount(() => {
  const stop = startSettings();
  if (initial.anchor) void show(initial.tab, initial.anchor);
  const offSection = inTauri()
    ? events.settingsSectionRequested.listen((e) => {
        const target = targetFor(e.payload);
        void show(target.tab, target.anchor);
      })
    : null;
  return () => {
    stop();
    void offSection?.then((off) => off());
  };
});
</script>

<div class="window">
  <nav>
    <h1>{t("settings-title")}</h1>
    <div role="tablist" aria-label={t("settings-tabs")} aria-orientation="vertical" tabindex="-1" onkeydown={keydown}>
      {#each TABS as id (id)}
        <button
          type="button"
          role="tab"
          id="tab-{id}"
          aria-selected={tab === id}
          aria-controls="tabpanel"
          tabindex={tab === id ? 0 : -1}
          onclick={() => show(id)}
        >
          {t(LABELS[id])}
        </button>
      {/each}
    </div>
  </nav>
  <div class="content" bind:this={content} id="tabpanel" role="tabpanel" aria-labelledby="tab-{tab}">
    {#if error}<p class="error" role="alert">{t(error)}</p>{/if}
    {#if tab === "general"}
      <GeneralTab {onerror} />
    {:else if tab === "sources"}
      <SourcesTab {onerror} />
    {:else if tab === "notifications"}
      <NotificationsTab {onerror} />
    {:else if tab === "privacy"}
      <PrivacyTab {onerror} />
    {:else}
      <AboutTab {onerror} />
    {/if}
  </div>
</div>

<style>
  .window {
    display: grid;
    grid-template-columns: 200px 1fr;
    height: 100vh;
  }
  nav {
    padding: var(--space-4) var(--space-2);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  h1 {
    font: var(--type-title);
    padding: 0 var(--space-2);
  }
  [role="tablist"] {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  [role="tab"] {
    position: relative;
    text-align: start;
    min-height: 32px;
    padding: 0 var(--space-3);
    border-radius: var(--radius-control);
    color: var(--text-1);
    transition: background-color var(--dur-fast) var(--ease-out);
  }
  [role="tab"]:hover {
    background: var(--surface-1);
  }
  [role="tab"][aria-selected="true"] {
    background: var(--surface-2);
    font-weight: 600;
  }
  /* The Windows 11 selection pill. */
  [role="tab"][aria-selected="true"]::before {
    content: "";
    position: absolute;
    left: 0;
    top: 8px;
    bottom: 8px;
    width: 3px;
    border-radius: var(--radius-pill);
    background: var(--accent);
  }
  .content {
    overflow-y: auto;
    padding: var(--space-4);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }
  .error {
    padding: var(--space-2) var(--space-3);
    border-left: 2px solid var(--signal-crit);
    background: var(--surface-1);
    border-radius: var(--radius-control);
  }
  @media (forced-colors: active) {
    [role="tab"][aria-selected="true"]::before {
      background: Highlight;
    }
  }
</style>

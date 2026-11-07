<!--
  The tray panel (Step 10): header, notices, limits, sessions and footer. Two views (ADR-048):
  the summary (ring, rows, sessions) and the detail (the Claude Code mod's report: pace, time
  strip, usage chart, costs), switched under the header and remembered between openings.

  Talks to the window lifecycle in src-tauri/src/panel/mod.rs:
  - After the first load it reports its content height (`panel_ready`); the window is sized,
    placed and shown only then, so the first visible frame is complete. Later size changes go
    through `panel_resize`.
  - `panel-shown` arrives each time the window appears: focus moves to the first control and
    the entrance (8 px rise and fade, design-language.md §6) plays once.
  - `Esc` hides the panel unless a tooltip consumed it.
-->
<script lang="ts">
import { onMount, tick } from "svelte";
import IconButton from "../../lib/components/IconButton.svelte";
import Skeleton from "../../lib/components/Skeleton.svelte";
import SourceBadge from "../../lib/components/SourceBadge.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import { commands, events, type PanelView, type SettingsSection } from "../../lib/ipc/bindings";
import { inTauri } from "../../lib/ipc/runtime";
import { clock } from "../../lib/time/clock.svelte";
import Icon from "./Icon.svelte";
import LimitsCard from "./LimitsCard.svelte";
import Notices from "./Notices.svelte";
import SessionsCard from "./SessionsCard.svelte";
import { load, panel, setView, startPanel } from "./state.svelte";
import {
  activeCount,
  emptyState,
  notices,
  planLabel,
  primaryLimit,
  type Refresh,
  refreshState,
} from "./view";

/** The entrance's start state is removed by `panel-shown`; this is the fallback. */
const ENTRANCE_FALLBACK_MS = 1500;

let content: HTMLDivElement;
let readySent = false;
/** Hidden until shown: the entrance transitions from here. */
let pre = $state(true);
let refreshClickedAt = $state<number | null>(null);
let fallback: ReturnType<typeof setTimeout> | undefined;

const VIEWS: readonly PanelView[] = ["summary", "detail"];
/** The detail view only has something to add when there are limits. */
const canSwitch = $derived(panel.snapshot != null && emptyState(panel.snapshot) == null);
const view = $derived<PanelView>(canSwitch ? panel.view : "summary");
let tabs: HTMLButtonElement[] = $state([]);

function other(v: PanelView): PanelView {
  return v === "summary" ? "detail" : "summary";
}

function onTabKey(e: KeyboardEvent) {
  if (e.key !== "ArrowLeft" && e.key !== "ArrowRight") return;
  e.preventDefault();
  const next = other(view);
  setView(next);
  tabs[VIEWS.indexOf(next)]?.focus();
}

const snapshot = $derived(panel.snapshot);
const plan = $derived(planLabel(snapshot?.account ?? null));
const active = $derived(snapshot ? activeCount(snapshot) : 0);
const primary = $derived(snapshot ? primaryLimit(snapshot) : null);
const refresh = $derived<Refresh>(
  snapshot ? refreshState(snapshot, clock.now, refreshClickedAt) : { shown: false },
);
const updated = $derived.by(() => {
  if (!primary) return "";
  const ms = clock.now - primary.observed_at;
  return ms < 60_000 ? t("source-updated-now") : t("source-updated", { ms });
});

function contentHeight(): number {
  return Math.ceil(content.getBoundingClientRect().height);
}

function openSettings(section: SettingsSection) {
  void commands.openSettings(section);
}

function refreshNow() {
  refreshClickedAt = Date.now();
  void commands.refreshLimits();
}

function armFallback() {
  clearTimeout(fallback);
  fallback = setTimeout(() => {
    pre = false;
  }, ENTRANCE_FALLBACK_MS);
}

function shown() {
  clearTimeout(fallback);
  pre = false;
  // The container, not the first button: that would open its tooltip on every open. Tab then
  // starts at the settings button.
  content.focus({ preventScroll: true });
  requestAnimationFrame(() => requestAnimationFrame(() => void commands.panelPainted()));
}

// The first load, rendered: the window may appear.
$effect(() => {
  if (!panel.loaded || readySent) return;
  readySent = true;
  void tick().then(() => {
    armFallback();
    if (inTauri()) void commands.panelReady(contentHeight());
    else pre = false;
  });
});

onMount(() => {
  const stop = startPanel();
  const resize = new ResizeObserver(() => {
    if (readySent && inTauri()) void commands.panelResize(contentHeight());
  });
  resize.observe(content);
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape" && !e.defaultPrevented && inTauri()) void commands.panelHide();
    if (e.key === "Tab" && e.ctrlKey && canSwitch) {
      e.preventDefault();
      setView(other(view));
    }
  };
  const onVisibility = () => {
    if (document.visibilityState === "hidden") {
      // Ready for the next entrance; a closed panel has no exit effect.
      pre = true;
      refreshClickedAt = null;
    } else {
      armFallback();
    }
  };
  window.addEventListener("keydown", onKey);
  document.addEventListener("visibilitychange", onVisibility);
  const offShown = inTauri() ? events.panelShown.listen(shown) : null;
  return () => {
    stop();
    resize.disconnect();
    clearTimeout(fallback);
    window.removeEventListener("keydown", onKey);
    document.removeEventListener("visibilitychange", onVisibility);
    void offShown?.then((off) => off());
  };
});

// A panel opened while the core was still starting has no snapshot yet: try again.
$effect(() => {
  if (!panel.loaded || panel.snapshot) return;
  const retry = setTimeout(() => void load(), 1000);
  return () => clearTimeout(retry);
});
</script>

<main>
  <div class="content" class:pre bind:this={content} tabindex="-1">
    <header>
      <div class="brand">
        <h1>{t("app-name")}</h1>
        {#if snapshot?.account}
          <span class="account">
            {plan
              ? t("panel-account", { label: snapshot.account.label, plan })
              : snapshot.account.label}
          </span>
        {/if}
      </div>
      {#if active > 0}
        <span class="active"><span class="live" aria-hidden="true"></span>{t("panel-active", { count: active })}</span>
      {/if}
      <span class="tip-end">
        <IconButton
          label={t("panel-open-settings")}
          placement="below"
          onclick={() => openSettings("general")}
        >
          <Icon name="gear" />
        </IconButton>
      </span>
    </header>

    {#if canSwitch}
      <div class="views" role="tablist" aria-label={t("panel-views")}>
        {#each VIEWS as v, i (v)}
          <button
            bind:this={tabs[i]}
            type="button"
            role="tab"
            id="tab-{v}"
            aria-selected={view === v}
            aria-controls="view"
            tabindex={view === v ? 0 : -1}
            onclick={() => setView(v)}
            onkeydown={onTabKey}
          >
            {t(v === "summary" ? "panel-view-summary" : "panel-view-detail")}
          </button>
        {/each}
      </div>
    {/if}

    {#if snapshot}
      <Notices notices={notices(snapshot)} {openSettings} />
      <div
        class="view"
        id="view"
        role={canSwitch ? "tabpanel" : undefined}
        aria-labelledby={canSwitch ? `tab-${view}` : undefined}
      >
        {#if view === "detail"}
          {#await import("./Detail.svelte") then { default: Detail }}
            <Detail {snapshot} detail={panel.detail} />
          {/await}
        {:else}
          <LimitsCard {snapshot} {openSettings} />
          {#if emptyState(snapshot) !== "no-claude"}
            <section aria-label={t("panel-sessions")}>
              <SessionsCard {snapshot} />
            </section>
          {/if}
        {/if}
      </div>
    {:else}
      <div class="loading" aria-busy="true">
        <span class="visually-hidden">{t("panel-loading")}</span>
        <span class="ring"><Skeleton width="120px" height="120px" /></span>
        <div class="lines">
          <Skeleton width="80%" />
          <Skeleton width="60%" />
          <Skeleton width="70%" />
        </div>
      </div>
    {/if}

    <footer>
      {#if primary}
        <SourceBadge source={primary.source} observedAt={primary.observed_at} />
        <span class="updated">{updated}</span>
      {/if}
      <span class="actions tip-end">
        {#if refresh.shown}
          <IconButton
            label={refresh.enabled
              ? t("panel-refresh")
              : refresh.reason === "cooldown"
                ? t("panel-refresh-wait")
                : t("panel-refresh-blocked")}
            disabled={!refresh.enabled}
            onclick={refreshNow}
          >
            <Icon name="refresh" />
          </IconButton>
        {/if}
        <IconButton
          label={panel.muted ? t("panel-unmute") : t("panel-mute")}
          aria-pressed={panel.muted}
          onclick={() => void commands.toggleMute()}
        >
          <Icon name={panel.muted ? "bell-off" : "bell"} />
        </IconButton>
      </span>
    </footer>
  </div>
</main>

<style>
  :global(html),
  :global(body) {
    height: 100%;
    overflow: hidden;
  }
  /* Without a height here `main` grows with its content and never scrolls: a view taller than
     the window (the detail view, many sessions) was cut off at the bottom. */
  :global(#app) {
    height: 100%;
  }
  main {
    height: 100%;
    overflow-y: auto;
    /* Thin, without arrow buttons: the classic bar takes a tenth of a 360 px panel. */
    scrollbar-width: thin;
    scrollbar-color: var(--text-3) transparent;
    color: var(--text-1);
  }
  .content {
    display: grid;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4) var(--space-3);
    transition:
      opacity var(--dur-base) var(--ease-out),
      translate var(--dur-base) var(--ease-out);
  }
  .content:focus {
    outline: none;
  }
  .content.pre {
    opacity: 0;
    translate: 0 8px;
    transition: none;
  }

  header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 28px;
  }
  .brand {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
  }
  h1 {
    margin: 0;
    font: var(--type-title);
  }
  .account,
  .active,
  .updated {
    font: var(--type-caption);
    color: var(--text-2);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .active {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }
  .live {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--signal-ok);
  }

  .views {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 2px;
    padding: 2px;
    border-radius: var(--radius-control);
    background: var(--surface-1);
    border: 1px solid var(--stroke);
  }
  .views button {
    appearance: none;
    border: 0;
    border-radius: calc(var(--radius-control) - 2px);
    padding: var(--space-1) var(--space-2);
    background: transparent;
    font: var(--type-body);
    color: var(--text-2);
    cursor: pointer;
  }
  .views button:hover {
    color: var(--text-1);
  }
  .views button[aria-selected="true"] {
    background: var(--track);
    color: var(--text-1);
    font-weight: 600;
  }
  .views button:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 1px;
  }
  .view {
    display: grid;
    gap: var(--space-3);
  }

  .loading {
    display: flex;
    gap: var(--space-3);
    align-items: center;
  }
  .ring {
    flex: none;
  }
  .ring :global(.skeleton) {
    border-radius: 50%;
  }
  .lines {
    flex: 1;
    display: grid;
    gap: var(--space-2);
  }

  footer {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 28px;
  }
  .updated {
    flex: 1;
    min-width: 0;
  }
  .actions {
    margin-left: auto;
    display: flex;
    gap: var(--space-1);
  }
  @media (forced-colors: active) {
    .live {
      background: CanvasText;
    }
    .views button[aria-selected="true"] {
      forced-color-adjust: none;
      background: Highlight;
      color: HighlightText;
    }
  }
</style>

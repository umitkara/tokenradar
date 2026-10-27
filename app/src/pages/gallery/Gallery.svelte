<!--
  Component gallery (development builds only). Every component in every state; theme and
  language switch here without touching the user's settings.
-->
<script lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import Button from "../../lib/components/Button.svelte";
import Card from "../../lib/components/Card.svelte";
import EmptyState from "../../lib/components/EmptyState.svelte";
import IconButton from "../../lib/components/IconButton.svelte";
import LimitBar from "../../lib/components/LimitBar.svelte";
import RingGauge from "../../lib/components/RingGauge.svelte";
import SessionRow from "../../lib/components/SessionRow.svelte";
import Skeleton from "../../lib/components/Skeleton.svelte";
import SourceBadge from "../../lib/components/SourceBadge.svelte";
import Toggle from "../../lib/components/Toggle.svelte";
import Tooltip from "../../lib/components/Tooltip.svelte";
import type { Locale } from "../../lib/i18n/format";
import { locale, setLocale, t } from "../../lib/i18n/i18n.svelte";
import type { Source, ThemePref } from "../../lib/ipc/bindings";
import { inTauri } from "../../lib/ipc/runtime";
import { applyTheme, settings } from "../../lib/settings.svelte";
import { bars as initialBars, nudge, ringCases, sessions } from "./fixtures";

let rings = $state(ringCases);
let bars = $state(initialBars);
let theme = $state<ThemePref>("system");
let estimates = $state(true);

const SOURCES: Source[] = ["statusline", "api", "estimate", "codex_log"];
const THEMES: ThemePref[] = ["system", "light", "dark"];
const LOCALES: Locale[] = ["en", "tr"];

function pickTheme(pref: ThemePref) {
  theme = pref;
  applyTheme(pref);
  // The backdrop follows the window theme; the gallery never persists it.
  if (inTauri()) void getCurrentWindow().setTheme(pref === "system" ? null : pref);
}

/** T, L and C cycle theme, language and values: quick to drive from a screenshot script. */
function onKey(e: KeyboardEvent) {
  if (e.ctrlKey || e.altKey || e.metaKey || e.target instanceof HTMLInputElement) return;
  const key = e.key.toLowerCase();
  if (key === "t") pickTheme(THEMES[(THEMES.indexOf(theme) + 1) % THEMES.length] ?? "system");
  else if (key === "l") setLocale(locale() === "en" ? "tr" : "en", settings.view.hour12);
  else if (key === "c") change();
}

function change() {
  rings = rings.map((r) => ({
    ...r,
    inner: r.inner && nudge(r.inner),
    outer: r.outer && nudge(r.outer),
  }));
  bars = bars.map(nudge);
}
</script>

<svelte:window onkeydown={onKey} />

<main>
  <header>
    <h1>{t("gallery-title")}</h1>
    <div class="controls">
      <div class="segmented" role="group" aria-label={t("gallery-theme")}>
        {#each THEMES as pref}
          <Button aria-pressed={theme === pref} onclick={() => pickTheme(pref)}>
            {t(`gallery-theme-${pref}`)}
          </Button>
        {/each}
      </div>
      <div class="segmented" role="group" aria-label={t("gallery-language")}>
        {#each LOCALES as l}
          <Button aria-pressed={locale() === l} onclick={() => setLocale(l, settings.view.hour12)}>
            {l.toLocaleUpperCase("en")}
          </Button>
        {/each}
      </div>
      <Button variant="primary" onclick={change}>{t("gallery-replay")}</Button>
    </div>
  </header>

  <section>
    <h2>{t("gallery-ring")}</h2>
    <div class="rings">
      {#each rings as ring (ring.key)}
        <figure>
          <RingGauge inner={ring.inner} outer={ring.outer} />
          <figcaption>{t(ring.key)}</figcaption>
        </figure>
      {/each}
    </div>
  </section>

  <div class="columns">
    <Card title={t("gallery-bars")}>
      {#snippet corner()}
        <SourceBadge source="statusline" observedAt={Date.now() - 120_000} />
      {/snippet}
      <div class="stack">
        {#each bars as bar, i (i)}
          <LimitBar limit={bar} countdown={bar.kind === "session" ? "relative" : "absolute"} />
        {/each}
      </div>
    </Card>

    <Card title={t("gallery-sessions")}>
      <div class="stack">
        {#each sessions as s (s.session_id)}
          <SessionRow session={s} />
        {/each}
      </div>
    </Card>

    <Card title={t("gallery-badges")}>
      <div class="row">
        {#each SOURCES as source}
          <SourceBadge {source} observedAt={Date.now() - 7 * 60_000} />
        {/each}
      </div>
    </Card>

    <Card title={t("gallery-controls")}>
      <div class="stack">
        <div class="row">
          <Button>{t("gallery-button")}</Button>
          <Button variant="primary">{t("gallery-button-primary")}</Button>
          <Button disabled>{t("gallery-button")}</Button>
          <IconButton label={t("gallery-icon-refresh")}>
            <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
              <path
                d="M13 8a5 5 0 1 1-1.5-3.55M13 2.5v2.5h-2.5"
                fill="none"
                stroke="currentColor"
                stroke-width="1.4"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </IconButton>
        </div>
        <Toggle bind:checked={estimates} label={t("gallery-toggle")} />
        <Tooltip text={t("source-updated", { ms: 420_000 })}>
          <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
          <span class="hint" tabindex="0">{t("gallery-tooltip")}</span>
        </Tooltip>
      </div>
    </Card>

    <Card title={t("gallery-states")}>
      <div class="stack">
        <Skeleton width="60%" />
        <Skeleton height="4px" />
        <Skeleton width="40%" />
        <EmptyState title={t("gallery-empty-title")} body={t("gallery-empty-body")}>
          {#snippet action()}
            <Button>{t("gallery-button")}</Button>
          {/snippet}
        </EmptyState>
      </div>
    </Card>
  </div>
</main>

<style>
  main {
    display: grid;
    gap: var(--space-5);
    padding: var(--space-5);
    color: var(--text-1);
  }
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-4);
  }
  h1 {
    margin: 0;
    font: var(--type-title);
  }
  h2 {
    margin: 0 0 var(--space-3);
    font: var(--type-title);
  }
  .controls {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
    margin-left: auto;
  }
  .segmented {
    display: flex;
    gap: 2px;
  }
  .segmented :global(button[aria-pressed="true"]) {
    border-color: var(--text-2);
  }
  .rings {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-5);
  }
  figure {
    display: grid;
    justify-items: center;
    gap: var(--space-2);
    margin: 0;
  }
  figcaption {
    font: var(--type-caption);
    color: var(--text-2);
  }
  .columns {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
    gap: var(--space-4);
    align-items: start;
  }
  .stack {
    display: grid;
    gap: var(--space-3);
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }
  .hint {
    font: var(--type-body);
    color: var(--text-2);
    text-decoration: underline dotted;
  }
</style>

<!--
  One project (Step 16b), `#projects/<id>`: all-time totals, its share of the last 8 weekly
  windows, model mix, the 5 most expensive sessions and every session (the sessions list with
  the project fixed). Everything comes from `get_project`; this lays it out.

  - Weekly shares are split from usage, so they are always `≈`. Weeks without a share yet
    (no attribution) are drawn by cost and the note says so.
  - The embedded sessions list keeps its filters in this page's fragment (`#projects/12?range=7d`).
-->
<script lang="ts">
import { onMount, untrack } from "svelte";
import Card from "../../../lib/components/Card.svelte";
import Skeleton from "../../../lib/components/Skeleton.svelte";
import { pct } from "../../../lib/i18n/format";
import { locale, t } from "../../../lib/i18n/i18n.svelte";
import {
  type CoreEvent,
  commands,
  type ProjectDetail,
  type SessionRow,
} from "../../../lib/ipc/bindings";
import { subscribeCoreEvents } from "../../../lib/ipc/core-events";
import { inTauri } from "../../../lib/ipc/runtime";
import { clock } from "../../../lib/time/clock.svelte";
import { usd } from "../../panel/detail";
import BarList, { type BarItem } from "../overview/BarList.svelte";
import { countOf, modelRows } from "../overview/view";
import { hrefFor } from "../router";
import SessionList from "../sessions/SessionList.svelte";
import { impactOf, modelLabel } from "../sessions/view";
import { projectListMemory } from "./filter";
import { weekBars } from "./view";

interface Props {
  id: number;
  /** The fragment's query: the embedded sessions list's filters. */
  query: string;
  /** Synthetic data for the gallery; skips the backend. */
  preset?: ProjectDetail | null;
  /** The gallery's rows for the embedded sessions list. */
  presetSessions?: SessionRow[] | null;
}

let { id, query, preset = null, presetSessions = null }: Props = $props();

let detail = $state<ProjectDetail | null>(untrack(() => preset));
let loaded = $state(untrack(() => preset) != null || !inTauri());
let failed = $state(false);

const RELEVANT = new Set<CoreEvent["type"]>(["usage_appended", "calibration_updated"]);
const MIN_GAP_MS = 10_000;
let lastLoad = 0;
let pending: ReturnType<typeof setTimeout> | null = null;

const backHref = $derived(hrefFor("projects", null, projectListMemory.query));
const weeks = $derived(detail ? weekBars(detail.weeks) : { bars: [], byCost: false });
const models = $derived<BarItem[]>(
  detail
    ? modelRows(detail.models).map((r) => ({
        key: r.key || "__rest",
        label: r.key ? modelLabel(r.key) : t("overview-model-rest"),
        width: r.width,
        value: money(r.value),
        title: `${r.key || t("overview-model-rest")} · ${pct(locale(), r.share)}`,
      }))
    : [],
);

async function load() {
  if (preset) return;
  lastLoad = Date.now();
  const result = await commands.getProject(id);
  if (result.status === "ok") {
    detail = result.data;
    failed = false;
  } else if (!detail) {
    failed = true;
  }
  loaded = true;
}

function schedule() {
  if (pending || preset) return;
  const wait = Math.max(0, lastLoad + MIN_GAP_MS - Date.now());
  pending = setTimeout(() => {
    pending = null;
    void load();
  }, wait);
}

onMount(() => {
  if (preset) return;
  void load();
  const stop = subscribeCoreEvents({
    onEvent: (e) => {
      if (RELEVANT.has(e.type)) schedule();
    },
    onResume: () => void load(),
  });
  return () => {
    stop();
    if (pending) clearTimeout(pending);
  };
});

function money(micro: number): string {
  return t("sessions-usd", { usd: usd(micro) });
}

function weekly(p: number | null): string {
  return p == null ? t("value-none") : t("value-pct", { pct: p, estimate: "yes" });
}

function ago(ms: number | null): string {
  if (ms == null) return t("value-none");
  const d = clock.now - ms;
  return d < 60_000 ? t("sessions-just-now") : t("sessions-ago", { ms: d });
}

function day(ms: number): string {
  return new Date(ms).toLocaleDateString(locale(), { day: "numeric", month: "short" });
}

function span(from: number, to: number): string {
  const f = { dateStyle: "medium", timeStyle: "short" } as const;
  return `${new Date(from).toLocaleString(locale(), f)} – ${new Date(to).toLocaleString(locale(), f)}`;
}
</script>

<nav class="crumbs" aria-label={t("session-breadcrumb")}>
  <a href={backHref}>← {t("project-back")}</a>
</nav>

{#if !loaded}
  <Card><Skeleton width="100%" height="120px" /></Card>
  <Card><Skeleton width="100%" height="200px" /></Card>
{:else if failed}
  <p class="error" role="alert">{t("main-load-failed")}</p>
{:else if !detail}
  <p class="error" role="alert">{t("project-not-found")}</p>
{:else}
  {@const p = detail.project}
  <header class="head">
    <h2>{p.name}</h2>
    <code class="path" title={p.path}>{p.path}</code>
    <dl class="stats">
      <div><dt>{t("project-cost")}</dt><dd class="readout">{money(p.cost)}</dd></div>
      <div><dt>{t("projects-col-sessions")}</dt><dd class="readout">{p.sessions}</dd></div>
      <div>
        <dt>{t("projects-col-last")}</dt>
        <dd class="readout" title={p.last_active != null ? new Date(p.last_active).toLocaleString(locale()) : undefined}>
          {ago(p.last_active)}
        </dd>
      </div>
      <div>
        <dt>{t("project-weekly-total")}</dt>
        <dd class="readout dim" title={t("projects-weekly-tip")}>{weekly(p.weekly_percent)}</dd>
      </div>
    </dl>
  </header>

  <Card title={t("project-weeks")}>
    {#if weeks.bars.length === 0}
      <p class="note">{t("project-weeks-empty")}</p>
    {:else}
      <ol class="weeks" aria-label={t("project-weeks")}>
        {#each weeks.bars as w, i (w.key)}
          <!-- Fewer than 8 weeks: the newest still sits at the right edge. -->
          <li style:grid-column-start={i === 0 ? 9 - weeks.bars.length : undefined} title="{span(w.starts, w.resets)} · {weekly(w.percent)} · {money(w.cost)}{w.estimated ? ` · ${t('project-week-estimated')}` : ''}">
            <span class="value readout" class:dim={w.estimated}>
              {weeks.byCost ? money(w.cost) : weekly(w.percent)}
            </span>
            <span class="track" aria-hidden="true">
              {#if w.height > 0}
                <span class="bar" class:estimated={w.estimated} style:height="max(2px, {w.height}%)"></span>
              {/if}
            </span>
            <span class="when">{day(w.starts)}</span>
          </li>
        {/each}
      </ol>
      <p class="note">{t(weeks.byCost ? "project-weeks-by-cost" : "project-weeks-note")}</p>
    {/if}
  </Card>

  <div class="pair">
    <Card title={t("overview-models")}>
      {#if models.length === 0}
        <p class="note">{t("project-models-empty")}</p>
      {:else}
        <BarList items={models} label={t("overview-models")} />
      {/if}
    </Card>

    <Card title={t("project-top")}>
      {#if detail.top_sessions.length === 0}
        <p class="note">{t("project-top-empty")}</p>
      {:else}
        <ol class="top">
          {#each detail.top_sessions as s (s.id)}
            {@const impact = impactOf(s)}
            <li>
              <a class="title" class:fallback={s.name.fallback} href={hrefFor("sessions", String(s.id))} title={s.name.text}>
                {s.name.text}
              </a>
              <span class="readout dim">
                {impact
                  ? t("value-pct", { pct: impact.percent, estimate: impact.kind === "weekly" || s.impact_basis?.approx ? "yes" : "no" })
                  : ""}
              </span>
              <span class="readout">{money(s.cost)}</span>
              <span class="readout dim">{t("overview-count", countOf(s.tokens.input + s.tokens.output + s.tokens.cache_read + s.tokens.cache_write))}</span>
            </li>
          {/each}
        </ol>
      {/if}
    </Card>
  </div>

  <section class="sessions" aria-label={t("project-sessions")}>
    <h3>{t("project-sessions")}</h3>
    <div class="list">
      <SessionList
        {query}
        locked={p.id}
        base={{ route: "projects", param: String(p.id) }}
        preset={presetSessions}
      />
    </div>
  </section>
{/if}

<style>
  .crumbs a {
    color: var(--text-2);
    text-decoration: none;
    font: var(--type-body);
  }
  .crumbs a:hover {
    color: var(--text-1);
    text-decoration: underline;
  }
  .head {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  h2 {
    margin: 0;
    font: var(--type-title);
    color: var(--text-1);
    overflow-wrap: anywhere;
  }
  .path {
    font: var(--type-caption);
    font-family: var(--font-mono);
    color: var(--text-2);
    overflow-wrap: anywhere;
  }
  .stats {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-5);
    margin: var(--space-2) 0 0;
  }
  .stats dt {
    font: var(--type-caption);
    color: var(--text-2);
  }
  .stats dd {
    margin: 0;
    font: var(--type-title);
    font-family: var(--font-mono);
    color: var(--text-1);
  }
  .stats dd.dim {
    color: var(--text-2);
  }
  .note {
    margin: var(--space-2) 0 0;
    font: var(--type-caption);
    color: var(--text-2);
  }
  .weeks {
    display: grid;
    grid-template-columns: repeat(8, minmax(0, 1fr));
    gap: var(--space-3);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .weeks li {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: var(--space-1);
    min-width: 0;
  }
  .weeks .value {
    text-align: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .track {
    display: flex;
    align-items: flex-end;
    justify-content: center;
    height: 96px;
    border-bottom: 1px solid var(--stroke);
  }
  .bar {
    width: min(100%, 28px);
    border-radius: 3px 3px 0 0;
    background: var(--series-2);
  }
  .bar.estimated {
    background: transparent;
    border: 1px dashed var(--series-2);
    border-bottom: 0;
  }
  .when {
    text-align: center;
    font: var(--type-caption);
    color: var(--text-2);
    white-space: nowrap;
  }
  .readout {
    font-size: 12px;
    font-family: var(--font-mono);
  }
  .dim {
    color: var(--text-2);
  }
  .pair {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: var(--space-4);
  }
  /* One grid for the whole list, so the number columns line up across rows. */
  .top {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    grid-template-columns: minmax(0, 1fr) max-content max-content max-content;
    gap: var(--space-2) var(--space-3);
    align-items: baseline;
    font: var(--type-body);
  }
  .top li {
    display: contents;
  }
  .top .readout {
    text-align: right;
  }
  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-1);
    text-decoration: none;
  }
  .title:hover {
    text-decoration: underline;
  }
  .fallback {
    color: var(--text-2);
  }
  .sessions h3 {
    margin: 0 0 var(--space-2);
    font: var(--type-title);
    color: var(--text-1);
  }
  .list {
    display: flex;
    flex-direction: column;
    height: 520px;
  }
  .error {
    padding: var(--space-2) var(--space-3);
    border-left: 2px solid var(--signal-crit);
    background: var(--surface-1);
    border-radius: var(--radius-control);
  }
</style>

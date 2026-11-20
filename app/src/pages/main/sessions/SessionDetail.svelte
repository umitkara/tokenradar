<!--
  One session (Step 16a), `#sessions/<id>`: what it was, what it cost over time, how much of each
  limit window it took, its subagents, the cache and where it ran. Everything comes from
  `get_session`; this lays it out. No conversation content anywhere: names are titles, folders
  come from the database.

  - Refreshes from usage events while visible (at most every 10 s), so a running session grows.
  - "Open folder" asks the backend, which only opens folders the database lists for this
    session (`open_session_dir`).
-->
<script lang="ts">
import { onMount, untrack } from "svelte";
import Card from "../../../lib/components/Card.svelte";
import { limitName } from "../../../lib/components/limit";
import Skeleton from "../../../lib/components/Skeleton.svelte";
import Tooltip from "../../../lib/components/Tooltip.svelte";
import { dur } from "../../../lib/i18n/format";
import { locale, t } from "../../../lib/i18n/i18n.svelte";
import {
  type CoreEvent,
  commands,
  type ImpactRow,
  type SessionDetail,
} from "../../../lib/ipc/bindings";
import { subscribeCoreEvents } from "../../../lib/ipc/core-events";
import { inTauri } from "../../../lib/ipc/runtime";
import { usd } from "../../panel/detail";
import { countOf } from "../overview/view";
import { hrefFor } from "../router";
import Badge from "./Badge.svelte";
import CostBars from "./CostBars.svelte";
import { listMemory } from "./filter";
import { costChart, durationOf, impactRows, modelLabel, shareStyle, windowBands } from "./view";

interface Props {
  id: number;
  /** Synthetic data for the gallery; skips the backend. */
  preset?: SessionDetail | null;
}

let { id, preset = null }: Props = $props();

// The gallery's preset is read once: it never changes for a mounted detail.
let detail = $state<SessionDetail | null>(untrack(() => preset));
let loaded = $state(untrack(() => preset) != null || !inTauri());
let failed = $state(false);
let dirError = $state<{ path: string; message: string } | null>(null);

const RELEVANT = new Set<CoreEvent["type"]>(["usage_appended", "calibration_updated"]);
const MIN_GAP_MS = 10_000;
let lastLoad = 0;
let pending: ReturnType<typeof setTimeout> | null = null;

const chart = $derived(detail ? costChart(detail.timeline, detail.bucket_ms) : null);
const bands = $derived(detail ? windowBands(detail.impact) : []);
const impact = $derived(detail ? impactRows(detail.impact) : []);
const backHref = $derived(hrefFor("sessions", null, listMemory.query));

async function load() {
  if (preset) return;
  lastLoad = Date.now();
  const result = await commands.getSession(id);
  if (result.status === "ok") {
    detail = result.data;
    failed = false;
  } else if (!detail) {
    failed = true;
  }
  loaded = true;
}

function schedule() {
  if (pending || !detail?.session.live) return;
  const wait = Math.max(0, lastLoad + MIN_GAP_MS - Date.now());
  pending = setTimeout(() => {
    pending = null;
    void load();
  }, wait);
}

onMount(() => {
  if (preset) return;
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

function count(n: number): string {
  return t("overview-count", countOf(n));
}

function share(percent: number | null, approx: boolean): string {
  if (percent == null) return t("value-none");
  return t("value-pct", { pct: percent, estimate: approx ? "yes" : "no" });
}

function hitText(r: ImpactRow): string {
  if (r.hit_limit == null) return t("session-impact-unknown");
  return r.hit_limit ? t("session-impact-yes") : t("session-impact-no");
}

async function openDir(path: string) {
  dirError = null;
  if (!inTauri()) return;
  const result = await commands.openSessionDir(id, path);
  if (result.status === "error") {
    dirError = {
      path,
      message: result.error === "missing" ? t("session-dir-missing") : t("session-dir-failed"),
    };
  }
}
</script>

<nav class="crumbs" aria-label={t("session-breadcrumb")}>
  <a href={backHref}>← {t("session-back")}</a>
</nav>

{#if !loaded}
  <Card><Skeleton width="100%" height="120px" /></Card>
  <Card><Skeleton width="100%" height="200px" /></Card>
{:else if failed}
  <p class="error" role="alert">{t("main-load-failed")}</p>
{:else if !detail}
  <p class="error" role="alert">{t("session-not-found")}</p>
{:else}
  {@const s = detail.session}
  {@const tokens = s.tokens.input + s.tokens.output + s.tokens.cache_read + s.tokens.cache_write}
  {@const duration = durationOf(s)}
  <header class="head">
    <h2 class:fallback={s.name.fallback}>
      {#if s.live}<span class="dot" title={t("sessions-running")}></span>{/if}
      {s.name.text}
    </h2>
    <div class="badges">
      {#if s.project}<Badge tone="accent">{s.project.name}</Badge>{/if}
      {#each s.models as m (m)}<Badge>{modelLabel(m)}</Badge>{/each}
      {#each detail.efforts as level (level)}<Badge>{t("session-effort", { level })}</Badge>{/each}
      {#if detail.fast}
        <Tooltip text={t("session-fast-tip")}>
          <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
          <span tabindex="0"><Badge>{t("session-fast")}</Badge></span>
        </Tooltip>
      {/if}
      {#if s.transcript_deleted}
        <Tooltip text={t("sessions-deleted-tip")}>
          <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
          <span tabindex="0"><Badge tone="muted">{t("sessions-deleted")}</Badge></span>
        </Tooltip>
      {/if}
    </div>
    <dl class="stats">
      <div><dt>{t("sessions-col-cost")}</dt><dd class="readout">{money(s.cost)}</dd></div>
      <div><dt>{t("session-tokens")}</dt><dd class="readout">{count(tokens)}</dd></div>
      <div>
        <dt>{t("sessions-col-duration")}</dt>
        <dd class="readout">{duration != null ? dur(locale(), duration) : t("value-none")}</dd>
      </div>
      <div><dt>{t("sessions-col-subagents")}</dt><dd class="readout">{s.subagents}</dd></div>
    </dl>
  </header>

  <Card title={t("session-chart")}>
    {#if chart && chart.xs.length > 0}
      <CostBars {chart} {bands} />
      <p class="note">{t("session-chart-bucket", { ms: detail.bucket_ms })}</p>
    {:else}
      <p class="note">{t("session-chart-empty")}</p>
    {/if}
  </Card>

  <Card title={t("session-impact")}>
    {#if impact.length === 0}
      <p class="note">{t("session-impact-empty")}</p>
    {:else}
      <table>
        <thead>
          <tr>
            <th scope="col">{t("session-impact-window")}</th>
            <th scope="col" class="num">{t("session-impact-share")}</th>
            <th scope="col" class="num">{t("session-impact-peak")}</th>
            <th scope="col">{t("session-impact-hit")}</th>
          </tr>
        </thead>
        <tbody>
          {#each impact as r (r.window_id)}
            {@const style = shareStyle(r.basis)}
            <tr class:weekly={r.kind !== "session"}>
              <th scope="row">
                <span class="window">{limitName(r)}</span>
                <span class="when">{new Date(r.starts_at).toLocaleString(locale(), { dateStyle: "medium", timeStyle: "short" })}</span>
              </th>
              <td class="num readout" class:dim={style.dim}>
                {share(r.percent, style.approx)}
                <span class="readout">· {money(r.cost)}</span>
              </td>
              <td class="num readout" class:dim={r.estimated}>{share(r.peak_percent, r.estimated)}</td>
              <td title={r.estimated ? t("session-impact-estimated") : undefined}>{hitText(r)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
      <p class="note">{t("session-impact-note")}</p>
    {/if}
  </Card>

  <div class="pair">
    <Card title={t("session-subagents")}>
      {#if detail.subagents.length === 0}
        <p class="note">{t("session-subagents-empty")}</p>
      {:else}
        <ul class="tree">
          {#each detail.subagents as a (a.id)}
            {@const at = a.tokens.input + a.tokens.output + a.tokens.cache_read + a.tokens.cache_write}
            <li>
              <span class="name" class:fallback={!a.title}>{a.title ?? t("session-subagent-untitled")}</span>
              <span class="models">{a.models.map(modelLabel).join(", ")}</span>
              <span class="readout">{count(at)}</span>
              <span class="readout">{money(a.cost)}</span>
            </li>
          {/each}
        </ul>
      {/if}
    </Card>

    <Card title={t("session-cache")}>
      <dl class="facts">
        <div>
          <dt>{t("session-cache-hit")}</dt>
          <dd class="readout">
            {detail.cache.hit_ratio != null ? share(detail.cache.hit_ratio * 100, false) : t("value-none")}
          </dd>
        </div>
        <div><dt>{t("session-cache-write-5m")}</dt><dd class="readout">{count(detail.cache.write_5m)}</dd></div>
        <div><dt>{t("session-cache-write-1h")}</dt><dd class="readout">{count(detail.cache.write_1h)}</dd></div>
        <div>
          <dt>{t("session-cache-savings")}</dt>
          <dd class="readout" title={t("session-cache-savings-tip")}>
            {t("session-cache-savings-value", { usd: usd(detail.cache.savings) })}
          </dd>
        </div>
      </dl>
    </Card>
  </div>

  <Card title={t("session-meta")}>
    <dl class="facts">
      <div><dt>{t("session-branch")}</dt><dd>{detail.git_branch ?? t("value-none")}</dd></div>
      <div><dt>{t("session-version")}</dt><dd>{detail.cc_version ?? t("value-none")}</dd></div>
      <div><dt>{t("session-entrypoint")}</dt><dd>{detail.entrypoint ?? t("value-none")}</dd></div>
      <div class="dirs">
        <dt>{t("session-dirs")}</dt>
        <dd>
          {#if detail.cwds.length === 0}
            {t("session-dirs-none")}
          {:else}
            <ul>
              {#each detail.cwds as cwd (cwd.path)}
                <li>
                  <code title={cwd.path}>{cwd.path}</code>
                  <button type="button" class="link" onclick={() => openDir(cwd.path)}>
                    {t("session-open-dir")}
                  </button>
                  {#if dirError?.path === cwd.path}
                    <span class="dir-error" role="status">{dirError.message}</span>
                  {/if}
                </li>
              {/each}
            </ul>
          {/if}
        </dd>
      </div>
    </dl>
  </Card>
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
  .fallback {
    color: var(--text-2);
  }
  .dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    margin-inline-end: var(--space-2);
    border-radius: 50%;
    background: var(--signal-ok);
    vertical-align: middle;
  }
  .badges {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1);
  }
  .stats {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-5);
    margin: var(--space-2) 0 0;
  }
  .stats dt,
  .facts dt {
    font: var(--type-caption);
    color: var(--text-2);
  }
  .stats dd {
    margin: 0;
    font: var(--type-title);
    font-family: var(--font-mono);
    color: var(--text-1);
  }
  .note {
    margin: var(--space-2) 0 0;
    font: var(--type-caption);
    color: var(--text-2);
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font: var(--type-body);
  }
  th,
  td {
    padding: var(--space-2) var(--space-2);
    text-align: start;
    border-bottom: 1px solid color-mix(in oklch, var(--stroke) 60%, transparent);
    vertical-align: baseline;
  }
  thead th {
    font: var(--type-caption);
    color: var(--text-2);
  }
  tbody th {
    font-weight: 400;
  }
  .num {
    text-align: end;
  }
  .window {
    color: var(--text-1);
  }
  .when {
    margin-inline-start: var(--space-2);
    font: var(--type-caption);
    color: var(--text-2);
  }
  tr.weekly .window {
    font-weight: 600;
  }
  .dim {
    color: var(--text-3);
  }
  .pair {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: var(--space-4);
  }
  .tree {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
  }
  .tree li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto auto;
    gap: var(--space-3);
    padding: var(--space-1) 0 var(--space-1) var(--space-4);
    border-left: 1px solid var(--stroke);
    font: var(--type-body);
  }
  .tree .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tree .models {
    font: var(--type-caption);
    color: var(--text-2);
  }
  .facts {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--space-2) var(--space-4);
    margin: 0;
  }
  .facts div {
    display: contents;
  }
  .facts dd {
    margin: 0;
    color: var(--text-1);
    min-width: 0;
  }
  .dirs ul {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .dirs li {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--space-2);
  }
  code {
    font: var(--type-caption);
    font-family: var(--font-mono);
    overflow-wrap: anywhere;
  }
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--text-1);
    text-decoration: underline;
    font: var(--type-caption);
    cursor: pointer;
  }
  .dir-error {
    font: var(--type-caption);
    color: var(--text-2);
  }
  .error {
    padding: var(--space-2) var(--space-3);
    border-left: 2px solid var(--signal-crit);
    background: var(--surface-1);
    border-radius: var(--radius-control);
  }
</style>

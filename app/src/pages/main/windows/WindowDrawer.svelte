<!--
  One window at `#windows/<id>` (Step 17a), as a modal side drawer over the page. Escape or
  "Close" goes back to the list with its filters.

  - Usage: the measured readings (none for an estimated window, and the note says so).
  - Session contributions: the four largest sessions and the rest, stacked, each share spread
    over time like its cost (`≈`). The share table under it is the same data as text, with
    links to the sessions; "Other" (claude.ai, Desktop) is a separate row because it has no
    time profile.
  - Notifications fired for this window, from the 30-day notification log.
-->
<script lang="ts">
import { onMount } from "svelte";
import Chart from "../../../lib/chart/Chart.svelte";
import type { ChartModel } from "../../../lib/chart/model";
import Skeleton from "../../../lib/components/Skeleton.svelte";
import { clock as clockText } from "../../../lib/i18n/format";
import { hour12, locale, t } from "../../../lib/i18n/i18n.svelte";
import { commands, type LimitKind, type Scope, type WindowDetail } from "../../../lib/ipc/bindings";
import { inTauri } from "../../../lib/ipc/runtime";
import { usd } from "../../panel/detail";
import { hrefFor } from "../router";
import { contribColor, contribSeries, nearestIndex, stackMax, stackPaths } from "./view";

interface Props {
  id: number;
  /** The list's query text, for "Close". */
  query: string;
  /** Synthetic data for the gallery; skips the backend. */
  preset?: WindowDetail | null;
  /** In the page flow, not modal (the gallery). */
  inline?: boolean;
}

let { id, query, preset = null, inline = false }: Props = $props();

const W = 600;
const H = 160;
let dialog: HTMLDialogElement;
let detail = $state<WindowDetail | null>(null);
let loaded = $state(false);
let hover = $state<number | null>(null);

onMount(() => {
  if (inline) dialog.show();
  else dialog.showModal();
  if (preset) {
    detail = preset;
    loaded = true;
    return;
  }
  if (!inTauri()) {
    loaded = true;
    return;
  }
  void commands.getWindow(id).then((r) => {
    detail = r.status === "ok" ? r.data : null;
    loaded = true;
  });
});

function close() {
  if (inline) return;
  location.hash = hrefFor("windows", null, query);
}

function money(micro: number): string {
  return t("sessions-usd", { usd: usd(micro) });
}

function value(p: number | null, approx: boolean): string {
  return p == null ? t("value-none") : t("value-pct", { pct: p, estimate: approx ? "yes" : "no" });
}

function when(ms: number): string {
  return new Date(ms).toLocaleString(locale(), {
    weekday: "short",
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
    hour12: hour12(),
  });
}

function limitName(kind: LimitKind, scope: Scope | null): string {
  if (kind === "session") return t("limit-five-hour");
  if (scope) return t("limit-weekly-scoped", { name: scope.model ?? scope.surface ?? "" });
  return t("limit-weekly");
}

const model = $derived.by((): ChartModel | null => {
  if (!detail || detail.points.length === 0) return null;
  const w = detail.window;
  return {
    from: w.starts_at,
    to: w.resets_at,
    yMax: 100,
    lines: [
      {
        id: "window",
        label: limitName(w.kind, w.scope),
        tone: w.kind === "session" ? "primary" : "secondary",
        points: detail.points.flatMap((p) =>
          p.percent == null ? [] : [{ at: p.at, value: p.percent }],
        ),
      },
    ],
    bands: [],
    thresholds: [75, 90],
  };
});

const stack = $derived(detail ? contribSeries(detail.contrib.series.map((s) => s.values)) : []);
const top = $derived(stackMax(stack));
const paths = $derived(stackPaths(stack, W, H, top));
const percentUnit = $derived(detail?.contrib.unit === "percent");

function contribValue(v: number): string {
  return percentUnit ? value(v, true) : money(v);
}

function seriesName(i: number): string {
  const s = detail?.contrib.series[i];
  if (!s) return "";
  return s.session_id == null ? t("window-contrib-rest") : (s.name?.text ?? "");
}

function onmove(e: PointerEvent) {
  const box = (e.currentTarget as SVGElement).getBoundingClientRect();
  hover = nearestIndex(
    (e.clientX - box.left) / Math.max(1, box.width),
    detail?.contrib.at.length ?? 0,
  );
}

function time(ms: number): string {
  const d = new Date(ms);
  return clockText(locale(), d.getHours(), d.getMinutes(), hour12());
}
</script>

<dialog bind:this={dialog} class:inline aria-labelledby="window-title" onclose={close}>
  {#if !loaded}
    <Skeleton width="100%" height="320px" />
  {:else if !detail}
    <p>{t("window-not-found")}</p>
  {:else}
    {@const w = detail.window}
    <header>
      <h2 id="window-title">
        {limitName(w.kind, w.scope)} · {when(w.starts_at)} – {when(w.resets_at)}
      </h2>
      <div class="facts">
        <span class="readout" class:dim={w.estimated}>{t("windows-col-peak")}: {value(w.peak_percent, w.estimated)}</span>
        <span class="readout">{t("windows-col-cost")}: {money(w.cost)}</span>
        <span class="readout">{t("windows-col-sessions")}: {w.sessions}</span>
        {#if w.estimated}<span class="tag">{t("windows-estimated")}</span>{/if}
      </div>
    </header>

    <section>
      <h3>{t("window-curve")}</h3>
      {#if model}
        <Chart {model} height={180} />
      {:else}
        <p class="note">{t("window-curve-none")}</p>
      {/if}
    </section>

    <section>
      <h3>{t("window-contrib")}</h3>
      {#if detail.contrib.series.length === 0 || top <= 0}
        <p class="note">{t("window-contrib-empty")}</p>
      {:else}
        <ul class="legend">
          {#each detail.contrib.series as s, i (s.session_id ?? "rest")}
            <li>
              <span class="swatch" style:background={contribColor(i, s.session_id != null)}></span>
              {seriesName(i)}
            </li>
          {/each}
        </ul>
        <div class="plot">
          <span class="ymax readout dim">{contribValue(top)}</span>
          <svg
            viewBox="0 0 {W} {H}"
            preserveAspectRatio="none"
            role="img"
            aria-label={t("window-contrib")}
            onpointermove={onmove}
            onpointerleave={() => (hover = null)}
          >
            {#each paths as d, i (i)}
              <path
                {d}
                fill={contribColor(i, detail.contrib.series[i]?.session_id != null)}
                fill-opacity="0.85"
                stroke="var(--surface-0-solid)"
                stroke-width="1"
                vector-effect="non-scaling-stroke"
              />
            {/each}
            {#if hover != null && detail.contrib.at.length > 1}
              {@const x = (hover / (detail.contrib.at.length - 1)) * W}
              <line x1={x} x2={x} y1="0" y2={H} class="cross" vector-effect="non-scaling-stroke" />
            {/if}
          </svg>
          {#if hover != null}
            {@const at = detail.contrib.at[hover]}
            <div class="tooltip" role="status">
              {#if at != null}<strong>{time(at)}</strong>{/if}
              {#each stack as series, i (i)}
                <span>{seriesName(i)}: {contribValue(series[hover] ?? 0)}</span>
              {/each}
            </div>
          {/if}
        </div>
        <p class="note">{t(percentUnit ? "window-contrib-note" : "window-contrib-cost")}</p>
      {/if}
    </section>

    <section>
      <h3>{t("window-shares")}</h3>
      {#if detail.shares.length === 0}
        <p class="note">{t("window-shares-empty")}</p>
      {:else}
        <table>
          <tbody>
            {#each detail.shares as s (s.session_id)}
              <tr>
                <td class="name">
                  <a href={hrefFor("sessions", String(s.session_id))}>{s.name.text}</a>
                  {#if s.project}<span class="dim"> · {s.project.name}</span>{/if}
                </td>
                <td class="num readout">{s.percent != null ? value(s.percent, true) : t("value-none")}</td>
                <td class="num readout dim">{money(s.cost)}</td>
              </tr>
            {/each}
            {#if detail.other?.percent != null}
              <tr>
                <td class="name">{t("window-share-other")}</td>
                <td class="num readout">{value(detail.other.percent, true)}</td>
                <td></td>
              </tr>
            {/if}
          </tbody>
        </table>
      {/if}
    </section>

    <section>
      <h3>{t("window-alerts")}</h3>
      {#if detail.alerts.length === 0}
        <p class="note">{t("window-alerts-empty")}</p>
      {:else}
        <ul class="alerts">
          {#each detail.alerts as a (`${a.rule}-${a.at}`)}
            <li><span class="readout dim">{when(a.at)}</span> {t("window-alert", { rule: a.rule, threshold: a.threshold ?? 0 })}</li>
          {/each}
        </ul>
      {/if}
    </section>
  {/if}
  <div class="actions">
    <button type="button" class="close" onclick={() => dialog.close()}>{t("window-close")}</button>
  </div>
</dialog>

<style>
  dialog {
    margin: 0 0 0 auto;
    width: min(680px, calc(100vw - 48px));
    max-width: none;
    height: 100vh;
    max-height: none;
    padding: var(--space-4);
    border: 0;
    border-left: 1px solid var(--stroke);
    background: var(--surface-0-solid);
    color: var(--text-1);
    overflow-y: auto;
  }
  dialog[open] {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }
  dialog::backdrop {
    background: oklch(0 0 0 / 0.3);
  }
  dialog.inline {
    position: static;
    margin: 0;
    width: 100%;
    height: auto;
    border: 1px solid var(--stroke);
    border-radius: var(--radius-card);
  }
  h2 {
    margin: 0;
    font: var(--type-title);
  }
  h3 {
    margin: 0 0 var(--space-2);
    font: var(--type-body-strong, var(--type-body));
  }
  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-4);
    margin-top: var(--space-2);
  }
  .readout {
    font-size: 12px;
    font-family: var(--font-mono);
    white-space: nowrap;
  }
  .dim {
    color: var(--text-2);
  }
  .tag {
    font: var(--type-caption);
    color: var(--text-2);
    border: 1px dashed var(--stroke);
    border-radius: 999px;
    padding: 0 var(--space-2);
  }
  .note {
    margin: var(--space-1) 0 0;
    font: var(--type-caption);
    color: var(--text-2);
  }
  .legend {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-3);
    margin: 0 0 var(--space-2);
    padding: 0;
    list-style: none;
    font: var(--type-caption);
  }
  .legend li {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }
  .swatch {
    width: 10px;
    height: 10px;
    border-radius: 2px;
  }
  .plot {
    position: relative;
  }
  svg {
    display: block;
    width: 100%;
    height: 160px;
    border-bottom: 1px solid var(--stroke);
  }
  .ymax {
    position: absolute;
    top: 0;
    left: 0;
    pointer-events: none;
  }
  .cross {
    stroke: var(--text-2);
    stroke-width: 1;
  }
  .tooltip {
    position: absolute;
    top: var(--space-1);
    right: var(--space-1);
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-2);
    border: 1px solid var(--stroke);
    border-radius: var(--radius-control);
    background: var(--surface-0-solid);
    font: var(--type-caption);
    pointer-events: none;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font: var(--type-body);
  }
  td {
    padding: var(--space-1) 0;
    border-bottom: 1px solid var(--stroke);
  }
  td.name {
    overflow-wrap: anywhere;
  }
  .num {
    text-align: right;
    padding-left: var(--space-3);
  }
  .alerts {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    margin-top: auto;
  }
  .close {
    min-height: 32px;
    padding: 0 var(--space-3);
    border-radius: var(--radius-control);
    border: 1px solid var(--stroke);
    background: var(--surface-1);
    color: var(--text-1);
    font: var(--type-body);
  }
  @media (forced-colors: active) {
    path {
      forced-color-adjust: none;
    }
  }
</style>

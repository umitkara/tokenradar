<!--
  Weekly cycles (Step 17a, ADR-070): one row per weekly window, newest first. Each row is a link
  to the window's drawer.

  - Days: seven 24-hour slices from the reset, as bars on one scale for the whole table. Values
    are `≈` (calibrated local use, no "Other"), so they need not add up to the peak; the note
    under the table says so. The day the limit was hit has a critical outline, and the
    "Hit the limit" column names it in words.
  - Scoped limits (Fable) get a column each; Anthropic's breakdown comes last.
-->
<script lang="ts">
import { pct, weekdayClock } from "../../../lib/i18n/format";
import { hour12, locale, t } from "../../../lib/i18n/i18n.svelte";
import type { CycleRow, Scope, WeekCycles } from "../../../lib/ipc/bindings";
import { usd } from "../../panel/detail";
import { surfaceKey } from "../overview/view";
import { hrefFor } from "../router";
import { barHeight, dayMax } from "./view";

interface Props {
  data: WeekCycles;
  /** The page's query text, kept on the drawer's link. */
  query: string;
}

let { data, query }: Props = $props();

const scale = $derived(dayMax(data.items));
const columns = $derived(
  `minmax(150px, auto) 176px 72px minmax(96px, auto) minmax(180px, 1fr)${" 84px".repeat(data.scopes.length)} minmax(140px, 0.7fr)`,
);

function money(micro: number): string {
  return t("sessions-usd", { usd: usd(micro) });
}

function value(p: number | null, approx: boolean): string {
  return p == null ? t("value-none") : t("value-pct", { pct: p, estimate: approx ? "yes" : "no" });
}

function dayName(ms: number): string {
  const d = new Date(ms);
  return weekdayClock(locale(), d.getDay(), d.getHours(), d.getMinutes(), hour12());
}

function weekSpan(from: number, to: number): string {
  const f = { day: "numeric", month: "short" } as const;
  return `${new Date(from).toLocaleDateString(locale(), f)} – ${new Date(to).toLocaleDateString(locale(), f)}`;
}

function scopeName(s: Scope): string {
  const name = s.model ?? s.surface ?? "";
  return t("limit-weekly-scoped", { name });
}

function hitText(r: CycleRow): string {
  if (r.window.hit_limit == null) return t("windows-hit-unknown");
  if (!r.window.hit_limit) return t("windows-hit-no");
  if (r.hit_at == null) return t("windows-hit-unknown");
  const d = new Date(r.hit_at);
  return weekdayClock(locale(), d.getDay(), d.getHours(), d.getMinutes(), hour12());
}

function dayTip(r: CycleRow, i: number): string {
  const d = r.days[i];
  if (!d) return "";
  const when = dayName(d.starts_at);
  if (d.future) return t("windows-day-future", { when });
  return t("windows-day-tip", { when, value: value(d.percent, true), cost: money(d.cost) });
}

function surfaces(r: CycleRow): string {
  return r.breakdown
    .filter((b) => (b.percent ?? 0) > 0)
    .map((b) => {
      const k = surfaceKey(b.key);
      return `${k ? t(k) : b.key} ${pct(locale(), b.percent ?? 0)}`;
    })
    .join(" · ");
}
</script>

<div class="table" role="table" aria-label={t("windows-tab-weeks")} style:--cols={columns}>
  <div class="head" role="row">
    <span role="columnheader">{t("windows-col-week")}</span>
    <span role="columnheader">{t("windows-col-days")}</span>
    <span role="columnheader" class="num">{t("windows-col-peak")}</span>
    <span role="columnheader">{t("windows-col-hit")}</span>
    <span role="columnheader">{t("windows-col-projects")}</span>
    {#each data.scopes as s (JSON.stringify(s))}
      <span role="columnheader" class="num">{scopeName(s)}</span>
    {/each}
    <span role="columnheader">{t("windows-col-surfaces")}</span>
  </div>
  {#each data.items as r (r.window.id)}
    {@const w = r.window}
    <a class="row" role="row" href={hrefFor("windows", String(w.id), query)}>
      <span role="cell" class="week">
        <span>{weekSpan(w.starts_at, w.resets_at)}</span>
        {#if w.open}<span class="tag">{t("windows-open")}</span>{/if}
        {#if w.estimated}<span class="tag dashed">{t("windows-estimated")}</span>{/if}
      </span>
      <span role="cell" class="days" aria-label={r.days.map((_, i) => dayTip(r, i)).join("; ")}>
        {#each r.days as d, i (d.starts_at)}
          {@const h = barHeight(scale.byCost ? d.cost : (d.percent ?? 0), scale.max)}
          <span class="day" class:future={d.future} class:hit={r.hit_day === i} title={dayTip(r, i)}>
            {#if h > 0}<span class="bar" class:est={w.estimated} style:height="max(2px, {h}%)"></span>{/if}
          </span>
        {/each}
      </span>
      <span role="cell" class="num readout" class:dim={w.estimated}>
        {value(w.peak_percent, w.estimated)}
      </span>
      <span role="cell" class="hit" class:crit={r.hit_at != null}>
        {#if r.hit_at != null}<span class="mark" aria-hidden="true">!</span>{/if}
        {hitText(r)}
      </span>
      <span role="cell" class="projects">
        {#each r.projects as p, i (p.project?.id ?? `none-${i}`)}
          <span class="project">
            <span class="name">{p.project?.name ?? t("windows-project-none")}</span>
            <span class="readout dim">{p.percent != null ? value(p.percent, true) : money(p.cost)}</span>
          </span>
        {/each}
      </span>
      {#each data.scopes as s (JSON.stringify(s))}
        {@const sp = r.scoped.find((x) => JSON.stringify(x.scope) === JSON.stringify(s))}
        <span role="cell" class="num readout" class:dim={sp?.estimated}>
          {sp ? value(sp.peak_percent, sp.estimated) : t("value-none")}{#if sp?.hit_limit} <span class="mark" aria-label={t("patterns-risk-hit")}>!</span>{/if}
        </span>
      {/each}
      <span role="cell" class="surfaces">{surfaces(r) || t("value-none")}</span>
    </a>
  {/each}
</div>
<p class="note">{t(scale.byCost ? "windows-days-by-cost" : "windows-days-note")}</p>

<style>
  .table {
    display: grid;
    grid-template-columns: var(--cols);
    overflow-x: auto;
    font: var(--type-body);
  }
  .head,
  .row {
    display: grid;
    grid-column: 1 / -1;
    grid-template-columns: subgrid;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
  }
  .head {
    font: var(--type-caption);
    color: var(--text-2);
    border-bottom: 1px solid var(--stroke);
  }
  .row {
    color: var(--text-1);
    text-decoration: none;
    border-bottom: 1px solid var(--stroke);
    min-height: 56px;
  }
  .row:hover {
    background: var(--surface-2);
  }
  .row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .week {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1) var(--space-2);
    white-space: nowrap;
  }
  .tag {
    font: var(--type-caption);
    color: var(--text-2);
    border: 1px solid var(--stroke);
    border-radius: 999px;
    padding: 0 var(--space-2);
  }
  .tag.dashed {
    border-style: dashed;
  }
  .days {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    gap: 3px;
    height: 36px;
  }
  .day {
    display: flex;
    align-items: flex-end;
    border-bottom: 1px solid var(--stroke);
    border-radius: 2px 2px 0 0;
  }
  .day.future {
    border-bottom-style: dotted;
  }
  .day.hit {
    outline: 1.5px solid var(--signal-crit);
    outline-offset: 1px;
  }
  .bar {
    width: 100%;
    background: var(--series-2);
    border-radius: 2px 2px 0 0;
  }
  .bar.est {
    background: transparent;
    border: 1px dashed var(--series-2);
    border-bottom: 0;
  }
  .num {
    text-align: right;
  }
  .readout {
    font-size: 12px;
    font-family: var(--font-mono);
    white-space: nowrap;
  }
  .dim {
    color: var(--text-2);
  }
  .hit {
    display: flex;
    align-items: center;
    gap: var(--space-1);
    white-space: nowrap;
    color: var(--text-2);
  }
  .hit.crit {
    color: var(--text-1);
  }
  .mark {
    display: inline-grid;
    place-items: center;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--signal-crit);
    color: var(--on-accent);
    font: 700 10px / 1 var(--font-ui, inherit);
  }
  .projects {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    max-width: 280px;
  }
  .project {
    display: flex;
    justify-content: space-between;
    gap: var(--space-2);
    min-width: 0;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .surfaces {
    font: var(--type-caption);
    color: var(--text-2);
  }
  .note {
    margin: var(--space-2) 0 0;
    font: var(--type-caption);
    color: var(--text-2);
  }
  @media (forced-colors: active) {
    .bar {
      background: CanvasText;
    }
    .day.hit {
      outline-color: Highlight;
    }
  }
</style>

<!--
  "What is using it?" (Step 15b): the 5 sessions and 5 projects with the largest share of this
  week's weekly limit, each with its `≈` share, API-equivalent cost and a daily-cost sparkline.
  Shares are always `≈` (attribution is an estimate) and dimmed while the calibration is not good
  enough; then a note says how they are split. Each name links to its detail page (Step 16b).
-->
<script lang="ts">
import Card from "../../../lib/components/Card.svelte";
import Sparkline from "../../../lib/components/Sparkline.svelte";
import { t } from "../../../lib/i18n/i18n.svelte";
import type { Overview } from "../../../lib/ipc/bindings";
import { hrefFor } from "../router";
import { dataState, usd } from "./view";

let { overview }: { overview: Overview } = $props();

interface Row {
  key: string;
  href: string;
  name: string;
  /** Project of a session, when it is not already the name. */
  sub: string | null;
  percent: number | null;
  cost: number;
  daily: number[];
}

const sessions = $derived<Row[]>(
  overview.top_sessions.map((s) => ({
    key: String(s.session_id),
    href: hrefFor("sessions", String(s.session_id)),
    name: s.name.text,
    sub: s.name.fallback ? null : (s.project?.name ?? null),
    percent: s.percent,
    cost: s.cost,
    daily: s.daily,
  })),
);
const projects = $derived<Row[]>(
  overview.top_projects.map((p) => ({
    key: String(p.project.id),
    href: hrefFor("projects", String(p.project.id)),
    name: p.project.name,
    sub: null,
    percent: p.percent,
    cost: p.cost,
    daily: p.daily,
  })),
);
const learning = $derived(dataState(overview).learning);
const dim = $derived(overview.basis?.dim ?? true);
const learningText = $derived.by(() => {
  if (overview.week.window_id == null) return t("overview-learning-no-week");
  if (learning == null) return null;
  if (learning === "none") return t("overview-learning-none");
  return t("overview-learning", { level: learning.level, n: learning.n_obs });
});
</script>

{#snippet list(title: string, rows: Row[])}
  <div class="list">
    <h3>{title}</h3>
    {#if rows.length === 0}
      <p class="note">{t("overview-week-empty")}</p>
    {:else}
      <ol>
        {#each rows as row (row.key)}
          <li>
            <span class="name">
              <a class="title" href={row.href} title={row.sub ? `${row.name} · ${row.sub}` : row.name}>{row.name}</a>
              {#if row.sub}<span class="sub">{row.sub}</span>{/if}
            </span>
            <span class="pct readout" class:dim title={t("overview-top-share")}>
              {row.percent == null ? t("value-none") : t("value-pct", { pct: row.percent, estimate: "yes" })}
            </span>
            <span class="cost readout" title={t("overview-top-cost")}>
              {t("overview-cost-value", { usd: usd(row.cost) })}
            </span>
            <span class="spark" title={t("overview-top-trend")}>
              <Sparkline values={row.daily} />
            </span>
          </li>
        {/each}
      </ol>
    {/if}
  </div>
{/snippet}

<Card title={t("overview-top")}>
  <div class="lists">
    {@render list(t("overview-top-sessions"), sessions)}
    {@render list(t("overview-top-projects"), projects)}
  </div>
  {#if learningText}
    <p class="note foot">{learningText}</p>
  {/if}
  {#if overview.basis?.stale}
    <p class="note foot">{t("overview-stale")}</p>
  {/if}
</Card>

<style>
  .list {
    container-type: inline-size;
    min-width: 0;
  }
  /* Too narrow for both: the session's own name wins; the project is in its tooltip. */
  @container (max-width: 420px) {
    .sub {
      display: none;
    }
  }
  .lists {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(min(300px, 100%), 1fr));
    gap: var(--space-5);
  }
  h3 {
    margin: 0 0 var(--space-2);
    font: var(--type-caption);
    font-weight: 600;
    color: var(--text-2);
  }
  ol {
    margin: 0;
    padding: 0;
    list-style: none;
    display: grid;
    grid-template-columns: minmax(0, 1fr) max-content max-content 64px;
    column-gap: var(--space-3);
  }
  li {
    display: contents;
  }
  li > span {
    padding: var(--space-1) 0;
    border-top: 1px solid var(--stroke);
    align-self: stretch;
    display: flex;
    align-items: center;
  }
  li:first-child > span {
    border-top: 0;
  }
  .name {
    min-width: 0;
    gap: var(--space-2);
  }
  .title {
    font: var(--type-body);
    color: var(--text-1);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-decoration: none;
  }
  .title:hover {
    text-decoration: underline;
  }
  .sub {
    flex-shrink: 1;
    min-width: 0;
    font: var(--type-caption);
    color: var(--text-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pct {
    justify-content: flex-end;
    color: var(--text-1);
  }
  .pct.dim {
    color: var(--text-3);
  }
  .cost {
    justify-content: flex-end;
    color: var(--text-2);
  }
  .note {
    margin: 0;
    font: var(--type-caption);
    color: var(--text-2);
  }
  .foot {
    margin-top: var(--space-3);
  }
</style>

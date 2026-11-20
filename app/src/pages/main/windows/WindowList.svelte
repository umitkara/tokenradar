<!--
  Every window of every kind in the range, newest first (Step 17a): keyset pages from
  `list_windows` in a virtual grid. Enter or a click opens the window's drawer.
-->
<script lang="ts">
import VirtualList from "../../../lib/components/VirtualList.svelte";
import { hour12, locale, t } from "../../../lib/i18n/i18n.svelte";
import type { LimitKind, Scope, WindowRow } from "../../../lib/ipc/bindings";
import { usd } from "../../panel/detail";
import { hrefFor } from "../router";

interface Props {
  rows: WindowRow[];
  hasMore: boolean;
  loading: boolean;
  onmore: () => void;
  query: string;
}

let { rows, hasMore, loading, onmore, query }: Props = $props();

const COLUMNS = "minmax(13em, 1.4fr) minmax(0, 1.3fr) minmax(0, 1.3fr) 6em 7em 5em 7em 6em";

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

function value(p: number | null, approx: boolean): string {
  return p == null ? t("value-none") : t("value-pct", { pct: p, estimate: approx ? "yes" : "no" });
}

function hit(r: WindowRow): string {
  if (r.hit_limit == null) return t("windows-hit-unknown");
  return r.hit_limit ? t("patterns-risk-hit") : t("windows-hit-no");
}
</script>

{#snippet header()}
  <span role="columnheader">{t("windows-col-limit")}</span>
  <span role="columnheader">{t("windows-col-start")}</span>
  <span role="columnheader">{t("windows-col-end")}</span>
  <span role="columnheader" class="num">{t("windows-col-peak")}</span>
  <span role="columnheader">{t("windows-col-hit")}</span>
  <span role="columnheader" class="num">{t("windows-col-sessions")}</span>
  <span role="columnheader" class="num">{t("windows-col-cost")}</span>
  <span role="columnheader" class="num">{t("windows-col-other")}</span>
{/snippet}

<VirtualList
  items={rows}
  key={(r) => r.id}
  label={t("windows-tab-list")}
  columns={COLUMNS}
  {header}
  {hasMore}
  {loading}
  {onmore}
  onactivate={(r) => (location.hash = hrefFor("windows", String(r.id), query))}
>
  {#snippet row(r: WindowRow)}
    <span role="gridcell" class="name">
      {limitName(r.kind, r.scope)}
      {#if r.open}<span class="tag">{t("windows-open")}</span>{/if}
      {#if r.estimated}<span class="tag dashed">{t("windows-estimated")}</span>{/if}
    </span>
    <span role="gridcell" class="muted">{when(r.starts_at)}</span>
    <span role="gridcell" class="muted">{when(r.resets_at)}</span>
    <span role="gridcell" class="num readout" class:muted={r.estimated}>{value(r.peak_percent, r.estimated)}</span>
    <span role="gridcell" class:crit={r.hit_limit === true}>{hit(r)}</span>
    <span role="gridcell" class="num readout">{r.sessions}</span>
    <span role="gridcell" class="num readout">{t("sessions-usd", { usd: usd(r.cost) })}</span>
    <span role="gridcell" class="num readout muted">{value(r.other_percent, true)}</span>
  {/snippet}
</VirtualList>

<style>
  .name {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
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
  .muted {
    color: var(--text-2);
  }
  .num {
    text-align: right;
  }
  .readout {
    font-size: 12px;
    font-family: var(--font-mono);
    white-space: nowrap;
  }
  .crit {
    color: var(--text-1);
    font-weight: 600;
  }
</style>

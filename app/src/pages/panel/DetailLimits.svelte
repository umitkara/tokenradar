<!--
  The detail view's limit cards (ADR-048), like the Claude Code mod's (ADR-046): each limit with
  its bar and time mark, its own source badge, its reset, its projection and its use against the
  time gone. The pace comes from the core (`PaceView`); estimates have none (ADR-044).
-->
<script lang="ts">
import Card from "../../lib/components/Card.svelte";
import LimitBar from "../../lib/components/LimitBar.svelte";
import { isEstimate } from "../../lib/components/limit";
import SourceBadge from "../../lib/components/SourceBadge.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import type { LimitView, PaceView, PanelDetail, PanelSnapshot } from "../../lib/ipc/bindings";
import { clock } from "../../lib/time/clock.svelte";
import Icon from "./Icon.svelte";
import { limitsOf } from "./view";

interface Props {
  snapshot: PanelSnapshot;
  detail: PanelDetail | null;
}

let { snapshot, detail }: Props = $props();

const limits = $derived(limitsOf(snapshot));
const rows = $derived(
  [limits.fiveHour, limits.weekly, ...limits.scoped].filter((l): l is LimitView => l != null),
);

const sameScope = (a: LimitView["scope"], b: LimitView["scope"]) =>
  (a?.model ?? a?.surface ?? null) === (b?.model ?? b?.surface ?? null);

function paceOf(limit: LimitView): PaceView | null {
  return detail?.pace.find((p) => p.kind === limit.kind && sameScope(p.scope, limit.scope)) ?? null;
}

function paceText(p: PaceView): string {
  const ahead = p.ahead ?? 0;
  const points = Math.round(Math.abs(ahead));
  const dir = points === 0 ? "even" : ahead > 0 ? "ahead" : "behind";
  // Whole percents: the time gone is read off the clock, a decimal adds nothing.
  return t("detail-pace", { elapsed: Math.round(p.elapsed_percent ?? 0), points, dir });
}

type Projected = { text: string; crit: boolean; low: boolean } | null;

function projectionOf(limit: LimitView): Projected {
  const p = limit.projection;
  if (!p || isEstimate(limit)) return null;
  const low = p.confidence === "low";
  if (p.will_hit_before_reset && p.eta_100_at != null) {
    const ms = Math.max(0, p.eta_100_at - clock.now);
    return { text: t("projection-full-in", { ms }), crit: true, low };
  }
  if (p.projected_at_reset != null) {
    return { text: t("projection-at-reset", { pct: p.projected_at_reset }), crit: false, low };
  }
  return null;
}
</script>

<Card>
  <h2 class="visually-hidden">{t("panel-limits")}</h2>
  <ul>
    {#each rows as limit (limit.kind + (limit.scope?.model ?? limit.scope?.surface ?? ""))}
      {@const pace = paceOf(limit)}
      {@const projected = projectionOf(limit)}
      <li class:scoped={limit.kind === "weekly_scoped"}>
        <LimitBar {limit} countdown={limit.kind === "session" ? "relative" : "absolute"} />
        <div class="meta">
          <SourceBadge source={limit.source} observedAt={limit.observed_at} />
          {#if pace}
            <span class="pace" class:warn={pace.warn}>{paceText(pace)}</span>
          {/if}
        </div>
        {#if projected}
          <p
            class="projection"
            class:crit={projected.crit}
            class:low={projected.low}
            title={projected.low ? t("panel-projection-low") : undefined}
          >
            {projected.text}
          </p>
        {/if}
      </li>
    {/each}
  </ul>
  {#if limits.fiveHourAbsent}
    <p class="extra"><Icon name="info" />{t("panel-no-five-hour")}</p>
  {/if}
</Card>

<style>
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-3);
  }
  li {
    display: grid;
    gap: var(--space-1);
  }
  .scoped .meta,
  .scoped .projection {
    padding-left: var(--space-4);
  }
  .meta {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
  }
  .pace,
  .projection,
  .extra {
    margin: 0;
    font: var(--type-caption);
    color: var(--text-2);
    font-variant-numeric: tabular-nums;
  }
  .pace {
    min-width: 0;
  }
  .pace.warn {
    color: var(--signal-warn);
    font-weight: 600;
  }
  .projection.crit {
    color: var(--signal-crit);
    font-weight: 600;
  }
  .projection.low {
    color: var(--text-3);
    font-weight: 400;
  }
  .extra {
    display: flex;
    gap: var(--space-1);
    align-items: center;
    margin-top: var(--space-3);
  }
  @media (forced-colors: active) {
    .pace.warn,
    .projection.crit {
      color: CanvasText;
    }
  }
</style>

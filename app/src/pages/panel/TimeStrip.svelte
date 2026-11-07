<!--
  The weekly time strip (ADR-046, ADR-048): two readings of the week on one strip. Above, the
  calendar days, cut at local midnight (dotted); the first and last are partial because the
  window starts and ends at the reset hour, which the legend under the chart states. In the strip, the use. From the reset, every 24 hours
  a solid line with the time line's value under it (14 %, 29 % …), and "▲ now".
  The 24-hour marks and their values come from the core; this places them.
-->
<script lang="ts">
import { weekdayName } from "../../lib/i18n/format";
import { locale, t } from "../../lib/i18n/i18n.svelte";
import type { FocusView, LimitView } from "../../lib/ipc/bindings";
import { clock } from "../../lib/time/clock.svelte";
import { clearOfNow, localMidnights, position, stripDays } from "./detail";

interface Props {
  focus: FocusView;
  limit: LimitView;
}

let { focus, limit }: Props = $props();

const start = $derived(focus.window_start);
const end = $derived(focus.resets_at);
const days = $derived(stripDays(start, end));
const midnights = $derived(localMidnights(start, end).map((at) => position(at, start, end)));
const periods = $derived(
  focus.periods.map((p) => ({ pos: position(p.at, start, end), value: p.time_percent ?? 0 })),
);
const nowPos = $derived(position(clock.now, start, end));
/** Near the right edge the word goes before the mark. */
const nowLeft = $derived(nowPos > 80);
const fill = $derived(Math.min(100, Math.max(0, limit.percent ?? 0)));

const dayLabel = (at: number) => weekdayName(locale(), new Date(at).getDay());

const pctText = (n: number) => t("value-pct", { pct: Math.round(n), estimate: "no" });
</script>

<div class="strip-block" aria-hidden="true">
  <div class="days">
    {#each days as day (day.at)}
      {#if day.label}
        <span class="day" style:left="{day.from}%">{dayLabel(day.at)}</span>
      {/if}
    {/each}
  </div>
  <div class="strip {limit.severity}">
    <span class="fill" style:width="{fill}%"></span>
    {#each midnights as pos (pos)}
      <span class="midnight" style:left="{pos}%"></span>
    {/each}
    {#each periods as p (p.pos)}
      <span class="period" style:left="{p.pos}%"></span>
    {/each}
    <span class="now-mark" style:left="{nowPos}%"></span>
  </div>
  <div class="below">
    {#each periods as p (p.pos)}
      {#if clearOfNow(p.pos, nowPos)}
        <span class="value" style:left="{p.pos}%">{pctText(p.value)}</span>
      {/if}
    {/each}
    <span class="now" class:left={nowLeft} style:left="{nowPos}%">
      {nowLeft ? `${t("detail-now")} ▲` : `▲ ${t("detail-now")}`}
    </span>
  </div>
</div>

<style>
  .strip-block {
    display: grid;
    gap: 2px;
  }
  .days,
  .below {
    position: relative;
    height: 14px;
    font: var(--type-caption);
    color: var(--text-2);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .day {
    position: absolute;
    top: 0;
    padding-left: 3px;
  }
  .strip {
    position: relative;
    height: 12px;
    border-radius: 3px;
    background: var(--track);
    overflow: hidden;
    color: var(--signal-ok);
  }
  .strip.warn {
    color: var(--signal-warn);
  }
  .strip.crit {
    color: var(--signal-crit);
  }
  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    background: currentColor;
  }
  .midnight,
  .period,
  .now-mark {
    position: absolute;
    top: 0;
    bottom: 0;
  }
  .midnight {
    border-left: 1px dotted var(--text-2);
  }
  .period {
    width: 1px;
    background: var(--text-2);
  }
  .now-mark {
    width: 2px;
    margin-left: -1px;
    background: var(--text-1);
    box-shadow: 0 0 0 1px var(--surface-1);
  }
  .value {
    position: absolute;
    top: 0;
    translate: -50% 0;
    color: var(--text-3);
  }
  .now {
    position: absolute;
    top: 0;
    translate: -4px 0;
    color: var(--text-1);
    font-weight: 600;
  }
  .now.left {
    translate: calc(-100% + 4px) 0;
  }
  @media (forced-colors: active) {
    .strip {
      border: 1px solid CanvasText;
    }
    .fill,
    .period,
    .now-mark {
      forced-color-adjust: none;
      background: CanvasText;
    }
    .now-mark {
      background: Highlight;
    }
  }
</style>

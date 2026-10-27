<!--
  One limit as a row (design-language.md §4): "[icon] Label ...... 72 % · 1h 48m" over a thin bar.
  Scoped limits ("Fable · weekly") are indented under their parent. Fill and ghost are clipped
  rather than scaled, so the dashes of an estimate keep their length.
-->
<script lang="ts">
import type { LimitView } from "../ipc/bindings";
import { clock } from "../time/clock.svelte";
import Countdown from "./Countdown.svelte";
import { arc, ariaSentence, isEstimate, limitName, projectedPercent, valueText } from "./limit";
import SeverityIcon from "./SeverityIcon.svelte";

interface Props {
  limit: LimitView;
  /** `absolute` for resets days away: "Resets Wed 10:59". */
  countdown?: "relative" | "absolute";
}

let { limit, countdown = "relative" }: Props = $props();

const name = $derived(limitName(limit));
const percent = $derived(arc(limit.percent));
const projected = $derived(projectedPercent(limit));
const estimated = $derived(isEstimate(limit));
</script>

<div
  class="row"
  class:scoped={limit.kind === "weekly_scoped"}
  class:stale={limit.freshness === "stale"}
  role="meter"
  aria-valuemin="0"
  aria-valuemax="100"
  aria-valuenow={limit.percent ?? undefined}
  aria-valuetext={ariaSentence(limit, name, clock.now)}
>
  <div class="line" aria-hidden="true">
    <SeverityIcon severity={estimated ? "ok" : limit.severity} />
    <span class="name">{name}</span>
    <span class="readout pct">{valueText(limit)}</span>
    {#if limit.resets_at != null}
      <span class="sep">·</span>
      <span class="when"><Countdown at={limit.resets_at} mode={countdown} /></span>
    {/if}
  </div>
  <div class="bar {estimated ? 'est' : limit.severity}" aria-hidden="true">
    {#if projected != null && projected > percent}
      <span
        class="ghost"
        class:over={projected > 100}
        style:clip-path="inset(0 {100 - arc(projected)}% 0 {percent}%)"
      ></span>
    {/if}
    <span class="fill" style:clip-path="inset(0 {100 - percent}% 0 0)"></span>
  </div>
</div>

<style>
  .row {
    display: grid;
    gap: var(--space-1);
  }
  .scoped {
    padding-left: var(--space-4);
  }
  .line {
    display: flex;
    align-items: baseline;
    gap: var(--space-1);
    font: var(--type-body);
    color: var(--text-2);
  }
  .line :global(svg) {
    align-self: center;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pct {
    color: var(--text-1);
    font-weight: 500;
  }
  .stale .pct {
    color: var(--text-3);
  }
  .sep,
  .when {
    font: var(--type-caption);
  }

  .bar {
    position: relative;
    height: 4px;
    border-radius: var(--radius-pill);
    background: var(--track);
    overflow: hidden;
    color: var(--signal-ok);
  }
  .scoped .bar {
    height: 3px;
  }
  .bar.warn {
    color: var(--signal-warn);
  }
  .bar.crit {
    color: var(--signal-crit);
  }
  .bar.est {
    color: var(--signal-est);
  }
  .stale .bar {
    opacity: 0.5;
  }
  .fill,
  .ghost {
    position: absolute;
    inset: 0;
    background: currentColor;
    transition: clip-path var(--dur-slow) var(--ease-out);
  }
  .est .fill {
    background: repeating-linear-gradient(90deg, currentColor 0 6px, transparent 6px 9px);
  }
  .ghost {
    opacity: 0.3;
  }
  .ghost.over {
    background: var(--signal-crit);
  }
  @media (forced-colors: active) {
    .bar {
      border: 1px solid CanvasText;
    }
    .fill {
      background: CanvasText;
    }
  }
</style>

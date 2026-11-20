<!--
  The 5-hour strip (Step 17a): one row per local day, newest first, the day's 24 hours left to
  right and each 5-hour window as a block (cut at midnight by the backend). The fill grows with
  the peak in the 5-hour colour (`--signal-ok`, ADR-066); a window that hit the limit has a
  critical outline, an estimated one a dashed outline. A block is a link to the window's drawer.

  Only the rows in view are rendered (`lib/virtual.ts`): a year of days stays cheap.
-->
<script lang="ts">
import { clock as clockText, pct } from "../../../lib/i18n/format";
import { hour12, locale, t } from "../../../lib/i18n/i18n.svelte";
import type { StripWindow, WindowStrip } from "../../../lib/ipc/bindings";
import { visibleRange } from "../../../lib/virtual";
import { hrefFor } from "../router";
import { blockBox } from "./view";

interface Props {
  data: WindowStrip;
  query: string;
}

let { data, query }: Props = $props();

const ROW = 30;
let scrollTop = $state(0);
let viewport = $state(480);

const range = $derived(visibleRange(scrollTop, viewport, ROW, data.days.length, 8));
const hours = [0, 3, 6, 9, 12, 15, 18, 21];

function time(ms: number): string {
  const d = new Date(ms);
  return clockText(locale(), d.getHours(), d.getMinutes(), hour12());
}

function peakText(w: StripWindow): string {
  const p = w.window.peak_percent;
  return p == null
    ? t("value-none")
    : t("value-pct", { pct: p, estimate: w.window.estimated ? "yes" : "no" });
}

function tip(w: StripWindow): string {
  const head = t("windows-block-tip", {
    from: time(w.window.starts_at),
    to: time(w.window.resets_at),
    peak: peakText(w),
    hit: w.window.hit_limit ? "yes" : "no",
  });
  const top = w.top
    .map((s) => `${s.name.text}${s.percent != null ? ` ${pct(locale(), s.percent)}` : ""}`)
    .join(", ");
  return top ? `${head}\n${top}` : head;
}

function fill(w: StripWindow): string {
  const p = Math.max(0, Math.min(100, w.window.peak_percent ?? 0));
  return `color-mix(in oklch, var(--signal-ok) ${Math.round(15 + p * 0.85)}%, transparent)`;
}

function dayLabel(ms: number): string {
  return new Date(ms).toLocaleDateString(locale(), {
    weekday: "short",
    day: "numeric",
    month: "short",
  });
}
</script>

<p class="legend">{t("windows-strip-legend")}</p>
<div class="axis" aria-hidden="true">
  <span></span>
  <span class="ticks">
    {#each hours as h (h)}
      <span style:left="{(h / 24) * 100}%">{clockText(locale(), h, 0, hour12())}</span>
    {/each}
  </span>
</div>
<div
  class="scroller"
  role="list"
  aria-label={t("windows-tab-strip")}
  bind:clientHeight={viewport}
  onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}
>
  <div class="spacer" style:height="{data.days.length * ROW}px">
    {#each data.days.slice(range.start, range.end) as d, i (d.day)}
      <div class="day" role="listitem" style:top="{(range.start + i) * ROW}px">
        <span class="label">{dayLabel(d.day)}</span>
        <span class="track">
          {#each d.segments as s (`${s.window}-${s.from_minute}`)}
            {@const w = data.windows[s.window]}
            {#if w}
              {@const box = blockBox(s.from_minute, s.to_minute, d.minutes)}
              <a
                class="block"
                class:hit={w.window.hit_limit === true}
                class:est={w.window.estimated}
                href={hrefFor("windows", String(w.window.id), query)}
                style:left="{box.left}%"
                style:width="{box.width}%"
                style:background={fill(w)}
                title={tip(w)}
                aria-label={tip(w)}
              ></a>
            {/if}
          {/each}
        </span>
      </div>
    {/each}
  </div>
</div>

<style>
  .legend {
    margin: 0 0 var(--space-2);
    font: var(--type-caption);
    color: var(--text-2);
  }
  .axis,
  .day {
    display: grid;
    grid-template-columns: 112px 1fr;
    gap: var(--space-3);
  }
  .ticks {
    position: relative;
    height: 16px;
    font: 400 11px / 16px var(--font-mono);
    color: var(--text-3);
  }
  .ticks span {
    position: absolute;
  }
  .scroller {
    flex: 1 1 0;
    min-height: 240px;
    overflow-y: auto;
  }
  .spacer {
    position: relative;
  }
  .day {
    position: absolute;
    inset-inline: 0;
    height: 30px;
    align-items: center;
  }
  .label {
    font: var(--type-caption);
    color: var(--text-2);
    white-space: nowrap;
  }
  .track {
    position: relative;
    height: 20px;
    border-radius: 2px;
    background: var(--surface-2);
  }
  .block {
    position: absolute;
    top: 0;
    bottom: 0;
    border-radius: 2px;
    border: 1px solid color-mix(in oklch, var(--signal-ok) 70%, transparent);
  }
  .block.est {
    border-style: dashed;
  }
  .block.hit {
    border: 2px solid var(--signal-crit);
  }
  .block:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  @media (forced-colors: active) {
    .block {
      background: Canvas !important;
      border-color: CanvasText;
    }
    .block.hit {
      border-color: Highlight;
    }
  }
</style>

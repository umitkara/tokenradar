<!--
  The dual ring (design-language.md §4): outer = weekly, inner = 5-hour window, both filling
  clockwise from 12 o'clock. Arcs are circles with `pathLength="100"`, so a percentage is a dash
  length and value changes are a CSS transition of `stroke-dasharray` (400 ms). A changed value
  also plays one radar sweep (600 ms). Nothing moves otherwise (ADR-019).
-->
<script lang="ts">
import { t } from "../i18n/i18n.svelte";
import type { LimitView, Severity } from "../ipc/bindings";
import { clock } from "../time/clock.svelte";
import Countdown from "./Countdown.svelte";
import { arc, ariaSentence, isEstimate, projectedPercent, valueText } from "./limit";

interface Props {
  /** The weekly limit (outer ring). */
  outer: LimitView | null;
  /** The 5-hour window (inner ring and the center readout). */
  inner: LimitView | null;
  /** Rendered width and height in px; the geometry is drawn at 168. */
  size?: number;
}

let { outer, inner, size = 168 }: Props = $props();

const uid = $props.id();
const C = 84;
/** Arc length of the critical notch and of the gap before it, in percent of the ring. */
const NOTCH = 1.5;

interface Ring {
  key: "outer" | "inner";
  r: number;
  width: number;
  limit: LimitView | null;
}

const rings = $derived<Ring[]>([
  { key: "outer", r: 77, width: 6, limit: outer },
  { key: "inner", r: 65, width: 10, limit: inner },
]);

/** Ticks at 25/50/75 % across both tracks, under the arcs. */
const TICKS = [25, 50, 75].map((p) => {
  const a = (p / 100) * 2 * Math.PI;
  const [r0, r1] = [60, 80];
  return {
    x1: C + r0 * Math.sin(a),
    y1: C - r0 * Math.cos(a),
    x2: C + r1 * Math.sin(a),
    y2: C - r1 * Math.cos(a),
  };
});

/** Always four entries so the dash array transitions smoothly between notch and no notch. */
function valueDash(percent: number, severity: Severity): string {
  if (severity === "crit" && percent > NOTCH * 2) {
    return `${percent - NOTCH * 2} ${NOTCH} ${NOTCH} 100`;
  }
  return `${percent} 0 0 100`;
}

function ghostDash(percent: number, projected: number): string {
  return `0 ${percent} ${Math.max(0, arc(projected) - percent)} 100`;
}

function tone(limit: LimitView | null): string {
  if (!limit) return "ok";
  return isEstimate(limit) ? "est" : limit.severity;
}

const label = $derived(
  t("aria-ring", {
    outer: ariaSentence(outer, t("limit-weekly"), clock.now),
    inner: ariaSentence(inner, t("limit-five-hour"), clock.now),
  }),
);

// One sweep per value change, never on mount.
let sweep = $state(0);
let last: string | undefined;
$effect(() => {
  const key = `${inner?.percent}|${outer?.percent}`;
  if (last !== undefined && key !== last) sweep++;
  last = key;
});
</script>

<div class="gauge" style:width="{size}px" style:height="{size}px" role="img" aria-label={label}>
  <svg viewBox="0 0 168 168" width={size} height={size} aria-hidden="true">
    <defs>
      {#each rings as ring (ring.key)}
        <mask id="{uid}-{ring.key}-dash" maskUnits="userSpaceOnUse">
          <circle
            cx={C}
            cy={C}
            r={ring.r}
            fill="none"
            stroke="#fff"
            stroke-width={ring.width + 2}
            pathLength="100"
            stroke-dasharray="1.6 0.9"
            transform="rotate(-90 {C} {C})"
          />
        </mask>
      {/each}
    </defs>

    {#each rings as ring (ring.key)}
      <circle class="track" cx={C} cy={C} r={ring.r} stroke-width={ring.width} />
    {/each}
    {#each TICKS as tick}
      <line class="tick" {...tick} />
    {/each}

    {#each rings as ring (ring.key)}
      {@const percent = arc(ring.limit?.percent ?? null)}
      {@const projected = projectedPercent(ring.limit)}
      {@const estimated = ring.limit ? isEstimate(ring.limit) : false}
      <g
        class="ring {tone(ring.limit)}"
        class:stale={ring.limit?.freshness === "stale"}
        transform="rotate(-90 {C} {C})"
      >
        {#if projected != null && projected > percent}
          <circle
            class="ghost"
            class:over={projected > 100}
            cx={C}
            cy={C}
            r={ring.r}
            stroke-width={ring.width}
            pathLength="100"
            stroke-dasharray={ghostDash(percent, projected)}
          />
        {/if}
        <circle
          class="value"
          cx={C}
          cy={C}
          r={ring.r}
          stroke-width={ring.width}
          pathLength="100"
          stroke-dasharray={valueDash(percent, ring.limit?.severity ?? "ok")}
          mask={estimated ? `url(#${uid}-${ring.key}-dash)` : undefined}
        />
      </g>
      {#if projected != null && projected > 100}
        <!-- Projected past 100 %: a ▲ where the ghost arc ends, at 12 o'clock. -->
        <path class="over-glyph" d="M{C} {C - ring.r - 3.5}l3.5 6h-7Z" />
      {/if}
    {/each}
  </svg>

  {#key sweep}
    {#if sweep > 0}<div class="sweep" aria-hidden="true"></div>{/if}
  {/key}

  <div class="center" aria-hidden="true">
    <span class="readout pct" class:dim={inner?.freshness === "stale"}>{valueText(inner)}</span>
    <span class="caption"><Countdown at={inner?.resets_at ?? null} mode="short" /></span>
  </div>
</div>

<style>
  .gauge {
    position: relative;
    flex: none;
  }
  svg {
    display: block;
    overflow: visible;
  }
  circle {
    fill: none;
  }
  .track {
    stroke: var(--track);
  }
  .tick {
    stroke: var(--text-3);
    stroke-width: 1;
    opacity: 0.7;
  }
  .ring {
    color: var(--signal-ok);
  }
  .ring.warn {
    color: var(--signal-warn);
  }
  .ring.crit {
    color: var(--signal-crit);
  }
  .ring.est {
    color: var(--signal-est);
  }
  .ring.stale {
    opacity: 0.5;
  }
  .value,
  .ghost {
    stroke: currentColor;
    transition: stroke-dasharray var(--dur-slow) var(--ease-out);
  }
  .ghost {
    opacity: 0.3;
  }
  .ghost.over {
    stroke: var(--signal-crit);
  }
  .over-glyph {
    fill: var(--signal-crit);
    stroke: var(--surface-0-solid);
    stroke-width: 1;
    paint-order: stroke;
  }

  .sweep {
    position: absolute;
    inset: 0;
    border-radius: 50%;
    background: conic-gradient(
      from 0deg,
      transparent 0 70%,
      color-mix(in oklch, var(--accent) 40%, transparent) 100%
    );
    mask: radial-gradient(circle, transparent 57px, #000 58px 81px, transparent 82px);
    opacity: 0;
    animation: sweep var(--dur-sweep) var(--ease-out) 1;
    pointer-events: none;
  }
  @keyframes sweep {
    from {
      rotate: 0turn;
      opacity: 1;
    }
    to {
      rotate: 1turn;
      opacity: 0;
    }
  }

  .center {
    position: absolute;
    inset: 0;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: 2px;
    pointer-events: none;
  }
  .pct {
    font: var(--type-display);
    color: var(--text-1);
    font-feature-settings: "tnum";
  }
  .pct.dim {
    color: var(--text-3);
  }
  .caption {
    font: var(--type-caption);
    color: var(--text-2);
  }

  @media (prefers-reduced-motion: reduce) {
    .sweep {
      display: none;
    }
  }
  @media (forced-colors: active) {
    .track {
      stroke: GrayText;
    }
    .ring,
    .ring.warn,
    .ring.crit,
    .ring.est,
    .ghost.over,
    .over-glyph {
      color: CanvasText;
      stroke: CanvasText;
    }
    .over-glyph {
      fill: CanvasText;
    }
  }
</style>

<!--
  One session's part in the 5-hour window (design-language.md §4), on two lines so it fits the
  360 px panel (ADR-032):
    ● name  project                ≈9 % this window
      Busy · 12.4k tok/min
  The name falls back to the transcript title, then the project. A busy dot pulses twice when
  the session turns busy, then stays still (ADR-019). Sessions that are not running are dimmed.
  The detail view (ADR-048) adds the API-equivalent cost to the second line and, when it has
  something to compare, a small bar: dashed for a ≈ share, solid for a part of the cost.
-->
<script lang="ts">
import { t } from "../i18n/i18n.svelte";
import type { LiveStatus, SessionShareView } from "../ipc/bindings";

interface Props {
  session: SessionShareView;
  /** Show the window's API-equivalent cost (detail view). */
  cost?: boolean;
  /** 0–100; `estimate` draws it dashed. */
  bar?: { width: number; estimate: boolean } | null;
}

let { session, cost = false, bar = null }: Props = $props();

const STATUS_KEYS: Record<LiveStatus, string> = {
  busy: "session-busy",
  idle: "session-idle",
  unknown: "session-unknown",
};

const name = $derived(session.name ?? session.title ?? session.project ?? t("session-untitled"));
/** The project, unless it already is the name. */
const project = $derived(
  session.project && (session.name ?? session.title) ? session.project : null,
);
const details = $derived(
  [
    session.status ? t(STATUS_KEYS[session.status]) : null,
    session.tokens_per_min ? t("session-rate", { k: session.tokens_per_min / 1000 }) : null,
    cost && session.cost > 0 ? t("detail-session-cost", { usd: session.cost / 1_000_000 }) : null,
  ].filter((d) => d != null),
);
</script>

<div class="row" class:ended={!session.status}>
  {#key session.status}
    <span class="dot {session.status ?? 'ended'}" aria-hidden="true"></span>
  {/key}
  <span class="who">
    <span class="name">{name}</span>
    {#if project}<span class="project">{project}</span>{/if}
  </span>
  {#if session.share_percent != null}
    <span class="share">{t("session-share", { pct: session.share_percent })}</span>
  {/if}
  {#if details.length > 0}
    <span class="details" title={cost && session.cost > 0 ? t("detail-session-cost-tip") : undefined}
      >{details.join(" · ")}</span
    >
  {/if}
  {#if bar}
    <span class="bar" class:est={bar.estimate} aria-hidden="true">
      <span class="fill" style:width="{bar.width}%"></span>
    </span>
  {/if}
</div>

<style>
  .row {
    display: grid;
    grid-template-columns: 8px minmax(0, 1fr) auto;
    column-gap: var(--space-2);
    align-items: baseline;
    font: var(--type-body);
    color: var(--text-2);
    min-height: 24px;
  }
  .ended {
    opacity: 0.7;
  }
  .dot {
    align-self: center;
    width: 8px;
    height: 8px;
    box-sizing: border-box;
    border-radius: 50%;
    border: 1.5px solid var(--text-3);
  }
  .dot.busy {
    border-color: var(--signal-ok);
    background: var(--signal-ok);
    animation: pulse 900ms var(--ease-out) 2;
  }
  .dot.idle {
    border-color: var(--text-2);
  }
  .dot.ended {
    border-style: dotted;
  }
  @keyframes pulse {
    50% {
      box-shadow: 0 0 0 4px color-mix(in oklch, var(--signal-ok) 30%, transparent);
    }
  }
  .who {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name {
    color: var(--text-1);
  }
  .project::before {
    content: " ";
  }
  .project {
    color: var(--text-3);
  }
  .share,
  .details {
    font: var(--type-caption);
    font-variant-numeric: tabular-nums;
  }
  .share {
    color: var(--text-1);
    white-space: nowrap;
  }
  .details {
    grid-column: 2 / -1;
    color: var(--text-3);
  }
  .bar {
    grid-column: 2 / -1;
    height: 3px;
    margin-top: var(--space-1);
    border-radius: var(--radius-pill);
    background: var(--track);
    overflow: hidden;
    color: var(--signal-ok);
  }
  .bar.est {
    color: var(--signal-est);
  }
  .fill {
    display: block;
    height: 100%;
    background: currentColor;
  }
  .est .fill {
    background: repeating-linear-gradient(90deg, currentColor 0 6px, transparent 6px 9px);
  }
  @media (prefers-reduced-motion: reduce) {
    .dot.busy {
      animation: none;
    }
  }
  @media (forced-colors: active) {
    .dot.busy {
      background: CanvasText;
    }
    .bar {
      border: 1px solid CanvasText;
    }
    .fill {
      forced-color-adjust: none;
      background: CanvasText;
    }
  }
</style>

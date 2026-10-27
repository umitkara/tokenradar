<!--
  One session's part in the 5-hour window (design-language.md §4):
  "● name (project)   Busy   ≈9 % this window   12.4k tok/min".
  A busy dot pulses twice when the session turns busy, then stays still (ADR-019).
-->
<script lang="ts">
import { t } from "../i18n/i18n.svelte";
import type { LiveStatus, SessionShareView } from "../ipc/bindings";

let { session }: { session: SessionShareView } = $props();

const STATUS_KEYS: Record<LiveStatus, string> = {
  busy: "session-busy",
  idle: "session-idle",
  unknown: "session-unknown",
};

const name = $derived(session.name ?? session.title ?? t("session-untitled"));
</script>

<div class="row">
  {#key session.status}
    <span class="dot {session.status ?? 'ended'}" aria-hidden="true"></span>
  {/key}
  <span class="who">
    <span class="name">{name}</span>
    {#if session.project}<span class="project">{session.project}</span>{/if}
  </span>
  {#if session.status}<span class="status">{t(STATUS_KEYS[session.status])}</span>{/if}
  {#if session.share_percent != null}
    <span class="share">{t("session-share", { pct: session.share_percent })}</span>
  {/if}
  {#if session.tokens_per_min != null}
    <span class="rate">{t("session-rate", { k: session.tokens_per_min / 1000 })}</span>
  {/if}
</div>

<style>
  .row {
    display: flex;
    align-items: baseline;
    gap: var(--space-2);
    font: var(--type-body);
    color: var(--text-2);
    min-height: 24px;
  }
  .dot {
    flex: none;
    align-self: center;
    width: 8px;
    height: 8px;
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
    flex: 1;
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
  .status,
  .share,
  .rate {
    flex: none;
    font: var(--type-caption);
    font-variant-numeric: tabular-nums;
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
  }
</style>

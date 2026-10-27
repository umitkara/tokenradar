<!--
  Calm notices above the limits (design-language.md §9): first indexing, a stale token, being
  offline and a suspected format change. Never in the error color.
-->
<script lang="ts">
import { t } from "../../lib/i18n/i18n.svelte";
import type { SettingsSection } from "../../lib/ipc/bindings";
import { clock } from "../../lib/time/clock.svelte";
import Icon from "./Icon.svelte";
import type { Notice } from "./view";

interface Props {
  notices: Notice[];
  openSettings: (section: SettingsSection) => void;
}

let { notices, openSettings }: Props = $props();
</script>

{#if notices.length > 0}
  <ul class="notices">
    {#each notices as notice (notice.type)}
      <li>
        {#if notice.type === "indexing"}
          <div class="indexing">
            <span>{t("panel-indexing", { pct: notice.pct })}</span>
            <div
              class="progress"
              role="progressbar"
              aria-label={t("panel-indexing", { pct: notice.pct })}
              aria-valuemin="0"
              aria-valuemax="100"
              aria-valuenow={notice.pct}
            >
              <span style:width="{notice.pct}%"></span>
            </div>
            {#if notice.recentReady}<span class="sub">{t("panel-indexing-ready")}</span>{/if}
          </div>
        {:else if notice.type === "token-stale"}
          <Icon name="info" /><span>{t("panel-token-stale")}</span>
        {:else if notice.type === "offline"}
          <Icon name="cloud-off" />
          <span>{t("panel-offline", { ms: Math.max(0, clock.now - (notice.since ?? clock.now)) })}</span>
        {:else if notice.type === "drift"}
          <Icon name="info" />
          <span>
            {t("panel-schema-drift")}
            <button type="button" class="link" onclick={() => openSettings("diagnostics")}>
              {t("panel-diagnostics")}
            </button>
          </span>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  .notices {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-2);
  }
  li {
    display: flex;
    gap: var(--space-2);
    align-items: flex-start;
    font: var(--type-caption);
    color: var(--text-2);
  }
  li :global(svg) {
    margin-top: -1px;
  }
  .indexing {
    flex: 1;
    display: grid;
    gap: var(--space-1);
  }
  .progress {
    height: 3px;
    border-radius: var(--radius-pill);
    background: var(--track);
    overflow: hidden;
  }
  .progress span {
    display: block;
    height: 100%;
    background: var(--accent);
  }
  .sub {
    color: var(--text-3);
  }
  .link {
    padding: 0;
    border: none;
    background: none;
    font: inherit;
    color: var(--text-1);
    text-decoration: underline;
    cursor: pointer;
  }
  @media (forced-colors: active) {
    .progress span {
      background: CanvasText;
    }
  }
</style>

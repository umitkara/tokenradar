<!--
  Running sessions by their ≈ share of the current window, at most five and "+N more". With
  nothing running, the window's three largest sessions, dimmed.
-->
<script lang="ts">
import Card from "../../lib/components/Card.svelte";
import SessionRow from "../../lib/components/SessionRow.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import type { PanelSnapshot } from "../../lib/ipc/bindings";
import { sessionList } from "./view";

let { snapshot }: { snapshot: PanelSnapshot } = $props();

const list = $derived(sessionList(snapshot));
const key = (s: (typeof list.rows)[number], i: number) => s.session_uuid ?? s.session_id ?? i;
</script>

<Card title={list.mode === "live" ? t("panel-live-sessions") : t("panel-top-sessions")}>
  {#if list.mode === "top"}
    <p class="none">{t("panel-no-live-sessions")}</p>
  {/if}
  {#if list.rows.length > 0}
    <ul>
      {#each list.rows as session, i (key(session, i))}
        <li><SessionRow {session} /></li>
      {/each}
    </ul>
  {/if}
  {#if list.more > 0}
    <p class="more">{t("panel-more", { count: list.more })}</p>
  {/if}
</Card>

<style>
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-2);
  }
  p {
    margin: 0;
    font: var(--type-caption);
    color: var(--text-2);
  }
  .none {
    font: var(--type-body);
  }
  .none:not(:last-child) {
    margin-bottom: var(--space-2);
  }
  .more {
    margin-top: var(--space-2);
    padding-left: calc(8px + var(--space-2));
  }
</style>

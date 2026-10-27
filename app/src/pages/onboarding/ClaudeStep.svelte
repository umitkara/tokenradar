<!--
  Step 2: the Claude Code folder and the first scan. Indexing already runs in the background
  (it starts with the app); the ring shows how far it got. Without a folder, a picker.
-->
<script lang="ts">
import Button from "../../lib/components/Button.svelte";
import Card from "../../lib/components/Card.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import { commands } from "../../lib/ipc/bindings";
import { inTauri } from "../../lib/ipc/runtime";
import { loadSources, store } from "../settings/state.svelte";
import { pickErrorKey } from "../settings/view";
import ProgressRing from "./ProgressRing.svelte";
import { loadSnapshot, onb } from "./state.svelte";
import { indexView } from "./view";

let { onerror }: { onerror: (key: string | null) => void } = $props();

let busy = $state(false);

const sources = $derived(store.sources);
const primary = $derived(sources?.dirs.find((d) => d.primary) ?? null);
const others = $derived(sources ? Math.max(0, sources.dirs.length - 1) : 0);
const index = $derived(indexView(onb.snapshot?.index_state ?? null));
const ringLabel = $derived(index.lines[0] ? t(index.lines[0].key, index.lines[0].args) : "");

async function pick() {
  if (!inTauri()) return;
  busy = true;
  const r = await commands.pickClaudeDir();
  busy = false;
  if (r.status === "error") {
    onerror(pickErrorKey(r.error));
    return;
  }
  onerror(null);
  if (r.data) await Promise.all([loadSources(), loadSnapshot()]);
}
</script>

{#if sources && !primary}
  <Card>
    <p class="state">{t("onb-claude-not-found")}</p>
    <p class="hint">{t("onb-claude-not-found-hint")}</p>
    <div class="actions">
      <Button variant="primary" disabled={busy} onclick={pick}>{t("sources-change")}</Button>
    </div>
  </Card>
{:else if primary}
  <Card>
    <p class="state">{t("onb-claude-found")}</p>
    <p class="path"><code>{primary.path}</code></p>
    {#if primary.stats}
      <p class="meta">
        {t("sources-dir-stats", {
          sessions: primary.stats.sessions,
          projects: primary.stats.projects,
          files: primary.stats.files,
        })}
      </p>
    {/if}
    {#if others > 0}<p class="meta">{t("onb-claude-more-dirs", { n: others })}</p>{/if}
    <div class="progress">
      <ProgressRing pct={index.pct} label={ringLabel} />
      <div aria-live="polite">
        {#each index.lines as line (line.key)}
          <p>{t(line.key, line.args)}</p>
        {/each}
      </div>
    </div>
    <div class="actions">
      <Button disabled={busy} onclick={pick}>{t("sources-change")}</Button>
    </div>
  </Card>
{/if}

<style>
  .state {
    font-weight: 600;
  }
  .hint,
  .meta {
    color: var(--text-2);
    margin-top: var(--space-1);
  }
  .meta {
    font: var(--type-caption);
  }
  .path {
    margin-top: var(--space-1);
  }
  code {
    font: 400 12px / 16px var(--font-mono);
    user-select: text;
    word-break: break-all;
  }
  .progress {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    margin-top: var(--space-4);
  }
  .actions {
    display: flex;
    gap: var(--space-2);
    margin-top: var(--space-4);
  }
</style>

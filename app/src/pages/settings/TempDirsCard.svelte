<!--
  Temporary folders (Step 16b): the patterns that hide throwaway projects on the projects page
  and in the sessions list's project picker. Each change is saved at once (`[data] temp_dirs`);
  the backend trims, drops repeats and applies them on the next list.
-->
<script lang="ts">
import Button from "../../lib/components/Button.svelte";
import Card from "../../lib/components/Card.svelte";
import IconButton from "../../lib/components/IconButton.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import { commands } from "../../lib/ipc/bindings";
import { savePrefs, store } from "./state.svelte";

let { onerror }: { onerror: (key: string | null) => void } = $props();

let draft = $state("");

const prefs = $derived(store.prefs);
const patterns = $derived(prefs?.data.temp_dirs ?? []);
const defaults = $derived(prefs?.default_temp_dirs ?? []);
const isDefault = $derived(
  patterns.length === defaults.length && patterns.every((p, i) => p === defaults[i]),
);

async function save(temp_dirs: string[]) {
  if (!prefs) return;
  const data = { ...prefs.data, temp_dirs };
  onerror((await savePrefs(() => commands.setDataPrefs(data))) ? null : "settings-save-failed");
}

async function add(e: SubmitEvent) {
  e.preventDefault();
  const text = draft.trim();
  if (!text) return;
  if (!patterns.some((p) => p.toLowerCase() === text.toLowerCase())) {
    await save([...patterns, text]);
  }
  draft = "";
}
</script>

<Card title={t("temp-dirs")}>
  <p class="hint">{t("temp-dirs-hint")}</p>
  {#if patterns.length === 0}
    <p class="hint">{t("temp-dirs-none")}</p>
  {:else}
    <ul class="patterns" aria-label={t("temp-dirs")}>
      {#each patterns as p (p)}
        <li>
          <code>{p}</code>
          <IconButton label={t("temp-dirs-remove", { pattern: p })} onclick={() => save(patterns.filter((x) => x !== p))}>
            <svg width="16" height="16" viewBox="0 0 16 16" aria-hidden="true">
              <path d="M4 4l8 8M12 4l-8 8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
            </svg>
          </IconButton>
        </li>
      {/each}
    </ul>
  {/if}
  <form class="add" onsubmit={add}>
    <input
      type="text"
      bind:value={draft}
      aria-label={t("temp-dirs-new")}
      placeholder={t("temp-dirs-placeholder")}
      spellcheck="false"
      autocomplete="off"
    />
    <Button type="submit" disabled={draft.trim() === ""}>{t("temp-dirs-add")}</Button>
    {#if !isDefault}
      <Button onclick={() => save([...defaults])}>{t("temp-dirs-reset")}</Button>
    {/if}
  </form>
</Card>

<style>
  .hint {
    font: var(--type-caption);
    color: var(--text-2);
    margin-top: var(--space-1);
  }
  .patterns {
    margin: var(--space-2) 0 0;
    padding: 0;
    list-style: none;
  }
  .patterns li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: 2px 0;
    border-top: 1px solid var(--stroke);
  }
  .patterns li:first-child {
    border-top: 0;
  }
  code {
    font: 400 12px / 16px var(--font-mono);
    user-select: text;
    word-break: break-all;
  }
  .add {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    margin-top: var(--space-2);
  }
  .add input {
    flex: 1 1 200px;
    min-height: 32px;
    padding: 0 var(--space-2);
    border-radius: var(--radius-control);
    border: 1px solid var(--stroke);
    background: var(--surface-1);
    color: var(--text-1);
    font: 400 12px / 16px var(--font-mono);
  }
  .add input::placeholder {
    color: var(--text-3);
    font-family: var(--font-ui, inherit);
  }
</style>

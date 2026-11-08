<!--
  "Add item": every kind the core knows but the line break, in groups, with what it shows and a
  sample; `title` names the line it adds to. A value
  item already in the line, a window the account does not have and a full list are offered
  but not addable, with the reason next to them.
-->
<script lang="ts">
import Button from "../../../lib/components/Button.svelte";
import { t } from "../../../lib/i18n/i18n.svelte";
import type { Item, SegmentKind, StatuslineKind } from "../../../lib/ipc/bindings";
import { groupKey, kindHintKey, kindKey, MAX_ITEMS, palette } from "../view";

interface Props {
  catalog: StatuslineKind[];
  items: Item[];
  title: string;
  onadd: (kind: SegmentKind) => void;
}

let { catalog, items, title, onadd }: Props = $props();

let dialog: HTMLDialogElement;
const groups = $derived(palette(catalog, items));
const full = $derived(items.length >= MAX_ITEMS);

export function open() {
  dialog.showModal();
}

function add(kind: SegmentKind) {
  dialog.close();
  onadd(kind);
}
</script>

<dialog bind:this={dialog} aria-labelledby="sl-add-title">
  <h2 id="sl-add-title">{title}</h2>
  {#if full}<p class="note">{t("statusline-add-full", { max: MAX_ITEMS })}</p>{/if}
  <div class="groups">
    {#each groups as group (group.group)}
      <section aria-labelledby="sl-group-{group.group}">
        <h3 id="sl-group-{group.group}">{t(groupKey(group.group))}</h3>
        <ul>
          {#each group.entries as entry (entry.kind)}
            {@const blocked = full || entry.used || entry.absent}
            <li>
              <button
                type="button"
                class="entry"
                disabled={blocked}
                aria-describedby="sl-hint-{entry.kind}"
                onclick={() => add(entry.kind)}
              >
                <span class="name">{t(kindKey(entry.kind))}</span>
                {#if entry.used}
                  <span class="tag">{t("statusline-add-used")}</span>
                {:else if entry.absent}
                  <span class="tag">{t("statusline-segment-absent")}</span>
                {/if}
                <span class="hint" id="sl-hint-{entry.kind}">{t(kindHintKey(entry.kind))}</span>
                {#if entry.sample}<code>{entry.sample}</code>{/if}
              </button>
            </li>
          {/each}
        </ul>
      </section>
    {/each}
  </div>
  <div class="actions">
    <Button onclick={() => dialog.close()}>{t("settings-cancel")}</Button>
  </div>
</dialog>

<style>
  dialog {
    width: min(560px, calc(100vw - 48px));
    max-height: calc(100vh - 48px);
    padding: var(--space-4);
    border: 1px solid var(--stroke);
    border-radius: var(--radius-card);
    background: var(--surface-0-solid);
    color: var(--text-1);
  }
  dialog::backdrop {
    background: oklch(0 0 0 / 0.4);
  }
  h2 {
    font: var(--type-title);
    margin-bottom: var(--space-3);
  }
  h3 {
    font: var(--type-caption);
    font-weight: 600;
    color: var(--text-2);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin: var(--space-3) 0 var(--space-1);
  }
  .groups section:first-child h3 {
    margin-top: 0;
  }
  ul {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: var(--space-1);
  }
  .entry {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: baseline;
    gap: 2px var(--space-2);
    width: 100%;
    height: 100%;
    padding: var(--space-2);
    border: 1px solid var(--stroke);
    border-radius: var(--radius-control);
    background: var(--surface-1);
    color: var(--text-1);
    font: var(--type-body);
    text-align: start;
    cursor: pointer;
  }
  .entry:hover:not(:disabled) {
    background: var(--surface-2);
  }
  .entry:disabled {
    cursor: default;
    color: var(--text-3);
  }
  .name {
    font-weight: 600;
  }
  .tag {
    font: var(--type-micro);
    padding: 1px var(--space-2);
    border-radius: var(--radius-pill);
    border: 1px solid var(--stroke);
    color: var(--text-2);
  }
  .hint {
    grid-column: 1 / -1;
    font: var(--type-caption);
    color: var(--text-2);
  }
  code {
    grid-column: 1 / -1;
    justify-self: start;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: pre;
    margin-top: 2px;
    padding: 1px var(--space-1);
    border-radius: 3px;
    background: var(--term-bg);
    color: var(--term-fg);
    font: 400 12px / 16px var(--font-term);
  }
  .entry:disabled code {
    opacity: 0.5;
  }
  .note {
    font: var(--type-caption);
    color: var(--text-2);
    margin-bottom: var(--space-2);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    margin-top: var(--space-4);
  }
</style>

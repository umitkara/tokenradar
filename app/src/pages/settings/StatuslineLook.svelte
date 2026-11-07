<!--
  What `tokenradar statusline` shows: the 12c segments and their order, the layout and the
  bar, mapped onto the item list (the Step 12j editor replaces this), with a preview the core draws on sample values (the same code the CLI runs). Every change sends the
  whole `StatuslinePrefs`; the core normalizes it and answers with what it kept.
-->
<script lang="ts">
import IconButton from "../../lib/components/IconButton.svelte";
import Select from "../../lib/components/Select.svelte";
import Toggle from "../../lib/components/Toggle.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import {
  commands,
  type Item,
  type SegmentKind,
  type StatuslinePrefs,
  type StatuslinePreview,
} from "../../lib/ipc/bindings";
import { inTauri } from "../../lib/ipc/runtime";
import Field from "./Field.svelte";
import { savePrefs, store } from "./state.svelte";
import {
  barOn,
  isDouble,
  moveSegment,
  paceOn,
  segmentLabelKey,
  segmentRows,
  setBar,
  setDouble,
  setPace,
  shownCount,
  toggleSegment,
} from "./view";

let { onerror }: { onerror: (key: string | null) => void } = $props();

const prefs = $derived(store.prefs?.statusline ?? null);
const rows = $derived(prefs ? segmentRows(prefs.items) : []);
const enabledCount = $derived(rows.filter((r) => r.enabled).length);
const lastShown = $derived(prefs ? shownCount(prefs.items) <= 1 : true);
const layouts = $derived<{ value: "single" | "double"; label: string }[]>([
  { value: "single", label: t("statusline-layout-single") },
  { value: "double", label: t("statusline-layout-double") },
]);

let preview = $state<StatuslinePreview>({ lines: [], absent: [] });

// Redrawn whenever the stored preferences or the language change.
$effect(() => {
  const p = prefs;
  void t("statusline-preview");
  if (!p || !inTauri()) return;
  let current = true;
  void commands.previewStatusline(p, "normal", null).then((lines) => {
    if (current) preview = lines;
  });
  return () => {
    current = false;
  };
});

async function save(items: Item[]) {
  if (!prefs) return;
  const next: StatuslinePrefs = { ...prefs, items };
  onerror(
    (await savePrefs(() => commands.setStatuslinePrefs(next))) ? null : "settings-save-failed",
  );
}

function toggle(kind: SegmentKind, on: boolean) {
  if (prefs) void save(toggleSegment(prefs.items, kind, on));
}

function move(index: number, by: -1 | 1) {
  if (prefs) void save(moveSegment(prefs.items, index, by));
}
</script>

{#if prefs}
  <div class="look">
    <h3>{t("statusline-look")}</h3>
    <p class="hint">{t("statusline-look-hint")}</p>

    <p class="caption">{t("statusline-preview")}</p>
    <div class="preview" aria-label={t("statusline-preview")} role="img">
      {#each preview.lines as line, i (i)}
        <div class="line">
          {#each line as span, j (j)}<span class={span.paint}>{span.text}</span>{/each}
        </div>
      {/each}
    </div>

    <p class="caption">{t("statusline-segments")}</p>
    <ul class="segments">
      {#each rows as row (row.kind)}
        <li class:off={!row.enabled}>
          <span class="name">
            <Toggle
              checked={row.enabled}
              label={t(segmentLabelKey(row.kind))}
              disabled={row.enabled && lastShown}
              onchange={(on) => toggle(row.kind, on)}
            />
            {#if preview.absent.includes(row.kind)}
              <span class="absent">{t("statusline-segment-absent")}</span>
            {/if}
          </span>
          {#if row.index !== null}
            {@const index = row.index}
            <span class="order">
              <IconButton
                label={t("statusline-move-up")}
                disabled={index === 0}
                onclick={() => move(index, -1)}
              >
                <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
                  <path d="M4 10l4-4 4 4" fill="none" stroke="currentColor" stroke-width="1.5" />
                </svg>
              </IconButton>
              <IconButton
                label={t("statusline-move-down")}
                disabled={index === enabledCount - 1}
                onclick={() => move(index, 1)}
              >
                <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true">
                  <path d="M4 6l4 4 4-4" fill="none" stroke="currentColor" stroke-width="1.5" />
                </svg>
              </IconButton>
            </span>
          {/if}
        </li>
      {/each}
    </ul>

    <Field label={t("statusline-layout")} id="statusline-layout">
      <Select
        id="statusline-layout"
        value={isDouble(prefs.items) ? "double" : "single"}
        options={layouts}
        onchange={(layout) => save(setDouble(prefs.items, layout === "double"))}
      />
    </Field>
    <Field label={t("statusline-bar")} hint={t("statusline-bar-hint")}>
      <Toggle
        checked={barOn(prefs.items)}
        label={t("statusline-bar")}
        labelHidden
        onchange={(bar) => save(setBar(prefs.items, bar))}
      />
    </Field>
    <Field label={t("statusline-pace")} hint={t("statusline-pace-hint")}>
      <Toggle
        checked={paceOn(prefs.items)}
        label={t("statusline-pace")}
        labelHidden
        onchange={(pace) => save(setPace(prefs.items, pace))}
      />
    </Field>
  </div>
{/if}

<style>
  .look {
    margin-top: var(--space-4);
    padding-top: var(--space-3);
    border-top: 1px solid var(--stroke);
  }
  h3 {
    font: var(--type-body);
    font-weight: 600;
  }
  .hint,
  .caption {
    font: var(--type-caption);
    color: var(--text-2);
    margin-top: var(--space-1);
  }
  .caption {
    margin-top: var(--space-3);
  }
  .preview {
    margin-top: var(--space-1);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-control);
    background: var(--surface-2);
    /* The terminal's own font: Geist Mono's subset has no block glyphs for the bar. */
    font: 400 12px / 18px "Cascadia Mono", Consolas, monospace;
    white-space: pre-wrap;
    user-select: text;
  }
  .plain {
    color: var(--text-1);
  }
  .dim {
    color: var(--text-3);
  }
  .ok {
    color: var(--signal-ok);
  }
  .warn {
    color: var(--signal-warn);
  }
  .crit {
    color: var(--signal-crit);
  }
  .segments li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 32px;
    gap: var(--space-2);
  }
  .segments li.off {
    color: var(--text-2);
  }
  .name {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .absent {
    font: var(--type-caption);
    color: var(--text-3);
  }
  .order {
    display: flex;
    gap: var(--space-1);
  }
</style>

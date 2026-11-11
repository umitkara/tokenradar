<!--
  Records a global shortcut: click, then press the keys. Escape cancels; a key the core does
  not take shows why and keeps listening. The text is written like the core's `Hotkey`.
-->
<script lang="ts">
import { t } from "../../lib/i18n/i18n.svelte";
import { hotkeyFromKey } from "./view";

interface Props {
  id: string;
  value: string;
  disabled?: boolean;
  onchange: (hotkey: string) => void;
}

let { id, value, disabled = false, onchange }: Props = $props();

let recording = $state(false);
let invalid = $state(false);

function start() {
  recording = true;
  invalid = false;
}

function stop() {
  recording = false;
  invalid = false;
}

function onkeydown(e: KeyboardEvent) {
  if (!recording) return;
  e.preventDefault();
  e.stopPropagation();
  if (e.code === "Escape" && !e.ctrlKey && !e.altKey && !e.metaKey) {
    stop();
    return;
  }
  const r = hotkeyFromKey(e);
  if (r.kind === "pending") return;
  if (r.kind === "invalid") {
    invalid = true;
    return;
  }
  stop();
  if (r.text !== value) onchange(r.text);
}
</script>

<div class="recorder">
  <button
    {id}
    type="button"
    class:recording
    {disabled}
    aria-describedby={invalid ? `${id}-invalid` : undefined}
    onclick={start}
    onblur={stop}
    {onkeydown}
  >
    {recording ? t("general-widget-hotkey-recording") : value}
  </button>
  {#if invalid}
    <p class="invalid" id="{id}-invalid" role="status">{t("general-widget-hotkey-invalid")}</p>
  {/if}
</div>

<style>
  .recorder {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
  }
  button {
    font: var(--type-body);
    font-variant-numeric: tabular-nums;
    min-height: 28px;
    min-width: 160px;
    padding: 0 var(--space-3);
    border-radius: var(--radius-control);
    border: 1px solid var(--stroke);
    background: var(--surface-2);
    color: var(--text-1);
    cursor: pointer;
  }
  button.recording {
    border-color: var(--accent);
    color: var(--text-2);
  }
  button:disabled {
    cursor: default;
    color: var(--text-3);
  }
  .invalid {
    font: var(--type-caption);
    color: var(--text-2);
    max-width: 240px;
    text-align: right;
  }
</style>

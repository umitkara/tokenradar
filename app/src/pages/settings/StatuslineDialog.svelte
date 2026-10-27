<!--
  Shows the `settings.json` change line by line before anything is written (ADR-009). Only the
  plan shown here can be applied: `apply_statusline` takes its token, and the core still refuses
  if the file changed in between. Closing the dialog discards the plan.
-->
<script lang="ts">
import { tick } from "svelte";
import Button from "../../lib/components/Button.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import {
  commands,
  type StatuslineAction,
  type StatuslineError,
  type StatuslinePlanView,
} from "../../lib/ipc/bindings";
import { statuslineErrorKey } from "./view";

interface Props {
  /** The user's current command; the chain option is offered only when there is one. */
  currentCommand: string | null;
  /** Called after the dialog closed; `backup` is set after a successful change. */
  ondone: (result: { backup: string | null } | null) => void;
}

let { currentCommand, ondone }: Props = $props();

let dialog: HTMLDialogElement;
let mode = $state<"install" | "uninstall">("install");
let chain = $state(true);
let plan = $state<StatuslinePlanView | null>(null);
let error = $state<string | null>(null);
let busy = $state(false);
let applied = false;

const action = $derived<StatuslineAction>(
  mode === "uninstall" ? "uninstall" : chain || !currentCommand ? "install" : "install_no_chain",
);
const ready = $derived(plan?.outcome === "ready" ? plan : null);

function fail(e: StatuslineError) {
  error = t(statuslineErrorKey(e));
}

async function makePlan() {
  busy = true;
  error = null;
  const r = await commands.planStatusline(action);
  busy = false;
  if (r.status === "ok") plan = r.data;
  else {
    plan = null;
    fail(r.error);
  }
}

/** Opens the dialog for `next` and plans the change. */
export async function open(next: "install" | "uninstall") {
  mode = next;
  chain = true;
  plan = null;
  applied = false;
  error = null;
  dialog.showModal();
  await tick();
  await makePlan();
}

async function setChain(e: Event & { currentTarget: HTMLInputElement }) {
  chain = e.currentTarget.checked;
  await makePlan();
}

async function confirm() {
  if (!ready) return;
  busy = true;
  error = null;
  const r = await commands.applyStatusline(ready.token);
  busy = false;
  if (r.status === "ok") {
    applied = true;
    dialog.close();
    ondone({ backup: r.data });
  } else {
    fail(r.error);
    // A file changed meanwhile or an outdated plan: show the current change instead.
    if (r.error.error === "changed_meanwhile" || r.error.error === "stale_plan") await makePlan();
  }
}

function closed() {
  if (!applied) {
    void commands.discardStatuslinePlan();
    ondone(null);
  }
  plan = null;
}
</script>

<dialog bind:this={dialog} onclose={closed} aria-labelledby="statusline-dialog-title">
  <h2 id="statusline-dialog-title">
    {t(mode === "install" ? "statusline-dialog-install" : "statusline-dialog-uninstall")}
  </h2>
  <p>{t("statusline-dialog-intro")}</p>

  {#if mode === "install" && currentCommand}
    <label class="chain">
      <input type="checkbox" checked={chain} disabled={busy} onchange={setChain} />
      <span>
        {t("statusline-chain")}
        <span class="hint">{t("statusline-chain-hint")}</span>
      </span>
    </label>
  {/if}

  {#if ready}
    <p class="path"><code>{ready.settings_path}</code></p>
    <!-- Focusable so the keyboard can scroll a long diff. -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <pre class="diff" aria-label={t("statusline-diff")} tabindex="0">{#each ready.diff as line, i (i)}<span class={line.kind}>{line.text}</span>{/each}</pre>
    {#if ready.chained_command}
      <p class="note">{t("statusline-chain-note")}</p>
    {/if}
    {#if ready.chained_mangled}
      <p class="note warn">{t("statusline-mangled")}</p>
    {/if}
    {#if ready.restores_previous}
      <p class="note">{t("statusline-restores")}</p>
    {/if}
  {:else if plan?.outcome === "already_installed"}
    <p class="note">{t("statusline-already")}</p>
  {:else if plan?.outcome === "not_installed"}
    <p class="note">{t("statusline-nothing")}</p>
  {/if}

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <div class="actions">
    <Button onclick={() => dialog.close()}>{t(ready ? "settings-cancel" : "settings-close")}</Button>
    {#if ready}
      <Button variant="primary" disabled={busy} onclick={confirm}>
        {t(mode === "install" ? "statusline-confirm-install" : "statusline-confirm-uninstall")}
      </Button>
    {/if}
  </div>
</dialog>

<style>
  dialog {
    width: min(640px, calc(100vw - 48px));
    max-height: calc(100vh - 48px);
    padding: var(--space-4);
    border: 1px solid var(--stroke);
    border-radius: var(--radius-card);
    background: var(--surface-0-solid);
    color: var(--text-1);
    display: none;
    flex-direction: column;
    gap: var(--space-3);
  }
  dialog[open] {
    display: flex;
  }
  dialog::backdrop {
    background: oklch(0 0 0 / 0.4);
  }
  h2 {
    font: var(--type-title);
  }
  .chain {
    display: flex;
    gap: var(--space-2);
    align-items: flex-start;
  }
  .hint {
    display: block;
    font: var(--type-caption);
    color: var(--text-2);
  }
  .path code {
    font: var(--type-caption);
    font-family: var(--font-mono);
    color: var(--text-2);
    user-select: text;
    word-break: break-all;
  }
  .diff {
    margin: 0;
    min-height: 80px;
    overflow: auto;
    padding: var(--space-2);
    border: 1px solid var(--stroke);
    border-radius: var(--radius-control);
    background: var(--surface-1);
    font: 400 12px / 16px var(--font-mono);
    user-select: text;
  }
  .diff span {
    display: block;
    width: max-content;
    min-width: 100%;
    white-space: pre;
  }
  .header,
  .hunk {
    color: var(--text-2);
  }
  .context {
    color: var(--text-1);
  }
  /* Signal colours mark the line, and the +/- prefix carries the meaning without colour. */
  .added {
    color: var(--text-1);
    background: color-mix(in oklch, var(--signal-ok) 22%, transparent);
  }
  .removed {
    color: var(--text-1);
    background: color-mix(in oklch, var(--signal-crit) 22%, transparent);
  }
  .note {
    font: var(--type-caption);
    color: var(--text-2);
  }
  .warn {
    color: var(--text-1);
    border-left: 2px solid var(--signal-warn);
    padding-left: var(--space-2);
  }
  .error {
    color: var(--text-1);
    border-left: 2px solid var(--signal-crit);
    padding-left: var(--space-2);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }
  @media (forced-colors: active) {
    .added,
    .removed {
      background: none;
    }
  }
</style>

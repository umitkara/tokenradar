<!--
  The terminal font helper (Step 12o, ADR-055). The terminal draws the statusline with its own
  font, so this dialog does what can be done from outside: it shows whether a Nerd Font is
  installed, downloads the pinned one after the user saw its size, source and licence, and
  selects it in Windows Terminal or VS Code through the same diff-and-confirm step as
  `settings.json` (ADR-009). Closing the dialog cancels a running download and forgets a plan.
-->
<script lang="ts">
import Button from "../../../lib/components/Button.svelte";
import { locale, t } from "../../../lib/i18n/i18n.svelte";
import {
  commands,
  type DiffLine,
  events,
  type FontChange,
  type FontError,
  type RestoreStep,
  type TargetId,
  type TerminalFontView,
} from "../../../lib/ipc/bindings";
import { inTauri } from "../../../lib/ipc/runtime";
import {
  canSelect,
  fontErrorKey,
  fontStatusKey,
  megabytes,
  percent,
  targetKey,
  targetTerminal,
  terminalKey,
} from "./font";

interface Props {
  /** Called whenever the view was (re)loaded, so the editor's preview can follow. */
  onchange: (view: TerminalFontView) => void;
}

let { onchange }: Props = $props();

interface ShownPlan {
  token: number;
  path: string;
  family: string;
  diff: DiffLine[];
  target: TargetId;
  change: FontChange;
}

let dialog: HTMLDialogElement;
let view = $state<TerminalFontView | null>(null);
/** The user asked to download and now sees what will be fetched. */
let asking = $state(false);
let progress = $state<{ done: number; total: number } | null>(null);
let plan = $state<ShownPlan | null>(null);
/** The one-click restore's summary, waiting for its single confirmation. */
let restore = $state<{ token: number; steps: RestoreStep[]; files: string[] } | null>(null);
let busy = $state(false);
let error = $state<string | null>(null);
let notice = $state<string | null>(null);

const font = $derived(view?.font ?? null);
const offerSize = $derived(view ? megabytes(view.offer.size_bytes, locale()) : "");

export async function open() {
  error = null;
  notice = null;
  asking = false;
  plan = null;
  restore = null;
  dialog.showModal();
  await reload();
}

async function reload() {
  if (!inTauri()) return;
  view = await commands.getTerminalFont();
  onchange(view);
}

function fail(e: FontError) {
  error = t(fontErrorKey(e), e.error === "status" ? { code: e.code } : undefined);
}

async function install() {
  if (!view) return;
  busy = true;
  error = null;
  notice = null;
  progress = { done: 0, total: view.offer.size_bytes };
  const off = events.fontInstallProgress.listen((e) => {
    progress = e.payload;
  });
  const r = await commands.installNerdFont();
  void off.then((stop) => stop());
  busy = false;
  progress = null;
  asking = false;
  if (r.status === "ok") {
    view = r.data;
    onchange(view);
    notice = t("font-installed", { family: view.offer.family });
  } else if (r.error.error !== "cancelled") {
    fail(r.error);
  }
}

async function makePlan(target: TargetId, change: FontChange) {
  busy = true;
  error = null;
  notice = null;
  const r = await commands.planTerminalFont(target, change);
  busy = false;
  if (r.status === "error") return fail(r.error);
  if (r.data.outcome === "unchanged") {
    notice = t(change === "select" ? "font-already-selected" : "font-nothing-to-undo");
    return;
  }
  const { token, settings_path, family, diff } = r.data;
  plan = { token, path: settings_path, family, diff, target, change };
}

async function confirmPlan() {
  if (!plan) return;
  const shown = plan;
  busy = true;
  error = null;
  const r = await commands.applyTerminalFont(shown.token);
  busy = false;
  if (r.status === "ok") {
    plan = null;
    const done = t(shown.change === "select" ? "font-selected" : "font-undone", {
      terminal: t(targetKey(shown.target)),
    });
    notice = `${done} ${t("mod-backup", { path: r.data })}`;
    await reload();
  } else {
    fail(r.error);
    // The file changed meanwhile or the plan is outdated: show the current change instead.
    if (r.error.error === "changed_meanwhile" || r.error.error === "stale_plan") {
      await makePlan(shown.target, shown.change);
    }
  }
}

async function makeRestore() {
  busy = true;
  error = null;
  notice = null;
  const r = await commands.planFontRestore();
  busy = false;
  if (r.outcome === "unchanged") {
    await reload();
    return;
  }
  restore = { token: r.token, steps: r.steps, files: r.font_files };
}

async function confirmRestore() {
  if (!restore) return;
  busy = true;
  error = null;
  const r = await commands.applyFontRestore(restore.token);
  busy = false;
  restore = null;
  if (r.status === "ok") notice = t("font-restored");
  else fail(r.error);
  await reload();
}

function backFromPlan() {
  plan = null;
  restore = null;
  void commands.discardStatuslinePlan();
}

function closed() {
  if (plan || restore) void commands.discardStatuslinePlan();
  if (progress) void commands.cancelNerdFont();
  plan = null;
  restore = null;
  asking = false;
}

function shownFont(step: RestoreStep, after: boolean): string {
  const font = (after ? step.to : step.from) ?? "";
  return after && step.to_default ? t("font-restore-default", { font }) : font;
}
</script>

<dialog bind:this={dialog} onclose={closed} aria-labelledby="font-dialog-title">
  <h2 id="font-dialog-title">{t("font-title")}</h2>

  {#if plan}
    <p>{t("statusline-dialog-intro")}</p>
    <p class="path"><code>{plan.path}</code></p>
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <pre class="diff" aria-label={t("statusline-diff")} tabindex="0">{#each plan.diff as line, i (i)}<span class={line.kind}>{line.text}</span>{/each}</pre>
    {#if plan.change === "select"}<p class="note">{t("font-restart-hint")}</p>{/if}
  {:else if restore}
    <p>{t("font-restore-intro")}</p>
    <ul class="steps">
      {#each restore.steps as step (step.target)}
        <li>
          <strong>{t(targetKey(step.target))}</strong>
          <span>{shownFont(step, false)} → {shownFont(step, true)}</span>
        </li>
      {/each}
      {#if restore.files.length > 0}
        <li>
          <strong>{t("font-restore-files", { count: restore.files.length })}</strong>
          {#each restore.files as file (file)}<code>{file}</code>{/each}
        </li>
      {/if}
    </ul>
    <p class="note">{t("font-restore-note")}</p>
  {:else}
    <p class="intro">{t("font-intro")}</p>
    {#if view}
      <section aria-labelledby="font-sec-font">
        <h3 id="font-sec-font">{t("font-section-font")}</h3>
        <p class:warn={!font}>{t(fontStatusKey(font), { family: font?.family ?? "" })}</p>
        {#if font && !font.mono}<p class="note">{t("font-wide-icons")}</p>{/if}
        {#if font?.tier !== "full"}
          {#if asking || busy}
            <div class="offer">
              <p><strong>{view.offer.family}</strong> · Nerd Fonts {view.offer.version} · {offerSize} · {view.offer.license}</p>
              <p class="path"><code>{view.offer.url}</code></p>
              <p class="note">{t("font-consent")}</p>
              {#if progress}
                <div class="progress">
                  <progress max={progress.total} value={progress.done} aria-label={t("font-downloading")}></progress>
                  <span>{t("font-progress", { percent: percent(progress.done, progress.total) })}</span>
                  <Button onclick={() => void commands.cancelNerdFont()}>{t("settings-cancel")}</Button>
                </div>
              {:else}
                <div class="actions">
                  <Button onclick={() => (asking = false)}>{t("settings-cancel")}</Button>
                  <Button variant="primary" disabled={busy} onclick={install}>{t("font-download-confirm")}</Button>
                </div>
              {/if}
            </div>
          {:else if view.can_install}
            <div class="actions start">
              <Button onclick={() => (asking = true)}>{t("font-download")}</Button>
            </div>
          {:else}
            <p class="note">{t("font-install-unsupported")}</p>
          {/if}
        {/if}
      </section>

      <section aria-labelledby="font-sec-terminal">
        <h3 id="font-sec-terminal">{t("font-section-terminal")}</h3>
        <p>
          {view.last_terminal
            ? t("font-terminal-seen", { terminal: t(terminalKey(view.last_terminal)) })
            : t("font-terminal-unknown")}
        </p>
      </section>

      <section aria-labelledby="font-sec-settings">
        <h3 id="font-sec-settings">{t("font-section-settings")}</h3>
        {#if view.targets.length === 0}
          <p class="note">{t("font-no-targets")}</p>
        {/if}
        <ul class="targets">
          {#each view.targets as target (target.id)}
            <li>
              <div class="row">
                <strong>{t(targetKey(target.id))}</strong>
                {#if view.last_terminal === targetTerminal(target.id)}
                  <span class="tag">{t("font-in-use")}</span>
                {/if}
                <span class="spacer"></span>
                {#if target.can_undo}
                  <Button disabled={busy} onclick={() => makePlan(target.id, "undo")}>{t("font-undo")}</Button>
                {/if}
                {#if canSelect(target, font)}
                  <Button disabled={busy} onclick={() => makePlan(target.id, "select")}>{t("font-select")}</Button>
                {/if}
              </div>
              {#if target.problem}
                <p class="note warn">{t(`font-problem-${target.problem}`)}</p>
              {:else}
                <p class="note">
                  {t(target.written ? "font-current" : "font-current-default", { font: target.font ?? "" })}
                  · {t(`font-tier-${target.tier}`)}
                </p>
                {#if target.overrides > 0}
                  <p class="note warn">{t("font-overrides", { count: target.overrides })}</p>
                {/if}
              {/if}
            </li>
          {/each}
        </ul>
        <details>
          <summary>{t("font-other-title")}</summary>
          <p class="note">{t("font-other-body", { family: font?.family ?? view.offer.family })}</p>
        </details>
      </section>

      {#if view.can_restore}
        <section aria-labelledby="font-sec-restore">
          <h3 id="font-sec-restore">{t("font-section-restore")}</h3>
          <p class="note">{t("font-restore-body")}</p>
          <div class="actions start">
            <Button disabled={busy} onclick={makeRestore}>{t("font-restore")}</Button>
          </div>
        </section>
      {/if}
    {/if}
  {/if}

  {#if notice}<p class="note done" role="status">{notice}</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <div class="actions">
    {#if plan}
      <Button onclick={backFromPlan}>{t("font-back")}</Button>
      <Button variant="primary" disabled={busy} onclick={confirmPlan}>
        {t(plan.change === "select" ? "font-confirm-select" : "font-confirm-undo")}
      </Button>
    {:else if restore}
      <Button onclick={backFromPlan}>{t("font-back")}</Button>
      <Button variant="primary" disabled={busy} onclick={confirmRestore}>{t("font-confirm-restore")}</Button>
    {:else}
      <Button onclick={() => dialog.close()}>{t("settings-close")}</Button>
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
    overflow: auto;
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
  h3 {
    font: var(--type-body);
    font-weight: 600;
    margin-bottom: var(--space-1);
  }
  section {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }
  .intro,
  .note {
    font: var(--type-caption);
    color: var(--text-2);
  }
  .warn {
    color: var(--text-1);
    border-left: 2px solid var(--signal-warn);
    padding-left: var(--space-2);
  }
  .done {
    color: var(--text-1);
    border-left: 2px solid var(--signal-ok);
    padding-left: var(--space-2);
  }
  .error {
    color: var(--text-1);
    border-left: 2px solid var(--signal-crit);
    padding-left: var(--space-2);
  }
  .offer {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-2);
    border: 1px solid var(--stroke);
    border-radius: var(--radius-control);
    background: var(--surface-1);
  }
  .path code {
    font: var(--type-caption);
    font-family: var(--font-mono);
    color: var(--text-2);
    user-select: text;
    word-break: break-all;
  }
  .progress {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .progress progress {
    flex: 1;
    accent-color: var(--accent);
  }
  .targets {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-height: 32px;
  }
  .spacer {
    flex: 1;
  }
  .tag {
    font: var(--type-micro);
    padding: 1px var(--space-2);
    border-radius: var(--radius-pill);
    border: 1px solid var(--stroke);
    color: var(--text-2);
  }
  .steps {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }
  .steps li {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .steps code {
    font: var(--type-caption);
    font-family: var(--font-mono);
    color: var(--text-2);
  }
  details summary {
    cursor: pointer;
    font: var(--type-caption);
    color: var(--text-2);
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
  .added {
    color: var(--text-1);
    background: color-mix(in oklch, var(--signal-ok) 22%, transparent);
  }
  .removed {
    color: var(--text-1);
    background: color-mix(in oklch, var(--signal-crit) 22%, transparent);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }
  .actions.start {
    justify-content: flex-start;
  }
  @media (forced-colors: active) {
    .added,
    .removed {
      background: none;
    }
  }
</style>

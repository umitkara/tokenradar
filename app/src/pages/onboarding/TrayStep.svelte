<!--
  Step 4: where the icon lives, "Always show the icon" (ADR-035), and starting with Windows.
  The pin is written only on the click; if Windows does not take it, the taskbar settings
  open with instructions. Autostart is applied when the user finishes.
-->
<script lang="ts">
import Button from "../../lib/components/Button.svelte";
import Card from "../../lib/components/Card.svelte";
import Toggle from "../../lib/components/Toggle.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import { commands } from "../../lib/ipc/bindings";
import { inTauri } from "../../lib/ipc/runtime";
import { settings } from "../../lib/settings.svelte";
import { onb } from "./state.svelte";
import TrayFigure from "./TrayFigure.svelte";
import { pinView } from "./view";

interface Props {
  autostart: boolean;
  onerror: (key: string | null) => void;
}

let { autostart = $bindable(), onerror }: Props = $props();

let tried = $state(false);
let busy = $state(false);

const view = $derived(pinView(onb.pin, settings.view.os, tried));

async function pin() {
  if (!inTauri()) return;
  busy = true;
  onb.pin = await commands.pinTrayIcon();
  tried = true;
  busy = false;
}

async function openTaskbarSettings() {
  if (!inTauri()) return;
  const r = await commands.openTaskbarSettings();
  onerror(r.status === "ok" ? null : "settings-save-failed");
}
</script>

<p class="intro">{t("onb-tray-intro")}</p>
<div class="figure"><TrayFigure label={t("onb-tray-figure")} /></div>

<Card>
  {#if view.status}<p class="state" role="status">{t(view.status.key)}</p>{/if}
  {#if view.offerPin}
    <div class="actions">
      <Button variant="primary" disabled={busy} onclick={pin}>{t("onb-tray-show")}</Button>
    </div>
  {/if}
  {#if view.manual}
    <p class="hint">{t(view.manual.key)}</p>
    <div class="actions">
      <Button onclick={openTaskbarSettings}>{t("onb-tray-open-settings")}</Button>
    </div>
  {/if}
  <div class="toggle">
    <Toggle bind:checked={autostart} label={t("onb-tray-autostart")} />
    <p class="meta">{t("onb-tray-autostart-hint")}</p>
  </div>
</Card>

<p class="meta">{t("onb-tray-done-hint")}</p>

<style>
  .intro,
  .hint {
    color: var(--text-2);
  }
  .figure {
    display: flex;
    justify-content: center;
  }
  .state {
    font-weight: 600;
  }
  .meta {
    font: var(--type-caption);
    color: var(--text-2);
    margin-top: var(--space-1);
  }
  .actions {
    display: flex;
    gap: var(--space-2);
    margin: var(--space-2) 0;
  }
  .toggle {
    margin-top: var(--space-3);
    padding-top: var(--space-3);
    border-top: 1px solid var(--stroke);
  }
</style>

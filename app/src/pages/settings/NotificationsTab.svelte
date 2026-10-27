<!--
  Notifications: thresholds, projection, last warning, resets, quiet hours and a test toast.
  Every change sends the whole `NotifyPrefs`; the core normalizes and answers with what it kept.
-->
<script lang="ts">
import Button from "../../lib/components/Button.svelte";
import Card from "../../lib/components/Card.svelte";
import Select from "../../lib/components/Select.svelte";
import Toggle from "../../lib/components/Toggle.svelte";
import { t } from "../../lib/i18n/i18n.svelte";
import { commands, type NotifyPrefs, type QuietHours } from "../../lib/ipc/bindings";
import Field from "./Field.svelte";
import { savePrefs, store } from "./state.svelte";
import ThresholdChips from "./ThresholdChips.svelte";
import { FINAL_WARNING_PRESETS, MAX_THRESHOLDS, optionsWith, RESET_PRESETS } from "./view";

let { onerror }: { onerror: (key: string | null) => void } = $props();

let testSent = $state(false);

const prefs = $derived(store.prefs?.notifications ?? null);
const off = $derived(prefs?.enabled === false);
const quiet = $derived<QuietHours>(prefs?.quiet_hours ?? {});

const finals = $derived(
  optionsWith(FINAL_WARNING_PRESETS, prefs?.final_warning_min ?? 15).map((m) => ({
    value: m,
    label: m === 0 ? t("notify-final-off") : t("notify-final-option", { ms: m * 60_000 }),
  })),
);
const resets = $derived(
  optionsWith(RESET_PRESETS, prefs?.reset_notify_min ?? 80).map((p) => ({
    value: p,
    label: p === 0 ? t("notify-reset-always") : t("notify-reset-option", { pct: p }),
  })),
);

async function save(patch: Partial<NotifyPrefs>) {
  if (!prefs) return;
  const next = { ...prefs, ...patch };
  onerror(
    (await savePrefs(() => commands.setNotificationPrefs(next))) ? null : "settings-save-failed",
  );
}

function setQuiet(patch: Partial<QuietHours>) {
  void save({ quiet_hours: { ...quiet, ...patch } });
}

function time(e: Event & { currentTarget: HTMLInputElement }): string | null {
  // An emptied time field keeps the stored value.
  return /^\d{2}:\d{2}$/.test(e.currentTarget.value) ? e.currentTarget.value : null;
}

async function test() {
  await commands.sendTestNotification();
  testSent = true;
}
</script>

{#if prefs}
  <Card>
    <Field label={t("notify-enabled")}>
      <Toggle
        checked={prefs.enabled ?? true}
        label={t("notify-enabled")}
        labelHidden
        onchange={(enabled) => save({ enabled })}
      />
    </Field>
  </Card>

  <Card>
    <p class="hint">{t("notify-thresholds-hint", { max: MAX_THRESHOLDS })}</p>
    <ThresholdChips
      id="five"
      label={t("notify-five")}
      values={prefs.five_hour_thresholds ?? []}
      disabled={off}
      onchange={(v) => save({ five_hour_thresholds: v })}
    />
    <ThresholdChips
      id="weekly"
      label={t("notify-weekly")}
      values={prefs.weekly_thresholds ?? []}
      disabled={off}
      onchange={(v) => save({ weekly_thresholds: v })}
    />
  </Card>

  <Card>
    <Field label={t("notify-projection")} hint={t("notify-projection-hint")}>
      <Toggle
        checked={prefs.projection ?? true}
        label={t("notify-projection")}
        labelHidden
        disabled={off}
        onchange={(projection) => save({ projection })}
      />
    </Field>
    <Field label={t("notify-final")} id="final">
      <Select
        id="final"
        value={prefs.final_warning_min ?? 15}
        options={finals}
        disabled={off || prefs.projection === false}
        onchange={(final_warning_min) => save({ final_warning_min })}
      />
    </Field>
    <Field label={t("notify-reset")} id="reset" hint={t("notify-reset-hint")}>
      <Select
        id="reset"
        value={prefs.reset_notify_min ?? 80}
        options={resets}
        disabled={off}
        onchange={(reset_notify_min) => save({ reset_notify_min })}
      />
    </Field>
  </Card>

  <Card>
    <Field label={t("notify-quiet")} hint={t("notify-quiet-hint")}>
      <Toggle
        checked={quiet.enabled ?? false}
        label={t("notify-quiet")}
        labelHidden
        disabled={off}
        onchange={(enabled) => setQuiet({ enabled })}
      />
    </Field>
    <div class="times">
      <label>
        {t("notify-quiet-from")}
        <input
          type="time"
          value={quiet.start ?? "23:00"}
          disabled={off || !quiet.enabled}
          onchange={(e) => {
            const start = time(e);
            if (start) setQuiet({ start });
          }}
        />
      </label>
      <label>
        {t("notify-quiet-to")}
        <input
          type="time"
          value={quiet.end ?? "08:00"}
          disabled={off || !quiet.enabled}
          onchange={(e) => {
            const end = time(e);
            if (end) setQuiet({ end });
          }}
        />
      </label>
    </div>
  </Card>

  <div class="test">
    <Button onclick={test}>{t("notify-test")}</Button>
    {#if testSent}<p class="hint" role="status">{t("notify-test-sent")}</p>{/if}
  </div>
{/if}

<style>
  .hint {
    font: var(--type-caption);
    color: var(--text-2);
  }
  .times {
    display: flex;
    gap: var(--space-4);
    padding-bottom: var(--space-2);
  }
  .times label {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }
  input[type="time"] {
    min-height: 28px;
    padding: 0 var(--space-2);
    border-radius: var(--radius-control);
    border: 1px solid var(--stroke);
    background: var(--surface-2);
    font-family: var(--font-mono);
  }
  input:disabled {
    opacity: 0.5;
  }
  .test {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
</style>

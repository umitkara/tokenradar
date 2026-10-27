<!--
  An on/off switch with its label. Space and Enter flip it, like any button. `labelHidden` keeps
  the label for screen readers only, for rows that show it elsewhere.
-->
<script lang="ts">
interface Props {
  checked: boolean;
  label: string;
  disabled?: boolean;
  labelHidden?: boolean;
  onchange?: (checked: boolean) => void;
}

let {
  checked = $bindable(),
  label,
  disabled = false,
  labelHidden = false,
  onchange,
}: Props = $props();

function flip() {
  checked = !checked;
  onchange?.(checked);
}
</script>

<button
  type="button"
  role="switch"
  aria-checked={checked}
  aria-label={labelHidden ? label : undefined}
  {disabled}
  onclick={flip}
>
  <span class="track" class:on={checked} aria-hidden="true"><span class="thumb"></span></span>
  {#if !labelHidden}<span>{label}</span>{/if}
</button>

<style>
  button {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: 0;
    border: 0;
    background: none;
    color: var(--text-1);
    font: var(--type-body);
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .track {
    position: relative;
    width: 32px;
    height: 16px;
    border-radius: var(--radius-pill);
    border: 1px solid var(--text-2);
    transition: background-color var(--dur-fast) var(--ease-out);
  }
  .thumb {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--text-2);
    transition: translate var(--dur-fast) var(--ease-out);
  }
  .on {
    background: var(--accent);
    border-color: var(--accent);
  }
  .on .thumb {
    translate: 16px 0;
    background: var(--on-accent);
  }
  @media (forced-colors: active) {
    .on {
      background: Highlight;
    }
    .on .thumb {
      background: HighlightText;
    }
  }
</style>

<!-- A native drop-down. `options` carry their own labels; the value keeps its type. -->
<script lang="ts" generics="T extends string | number">
interface Option {
  value: T;
  label: string;
}

interface Props {
  id?: string;
  value: T;
  options: readonly Option[];
  disabled?: boolean;
  /** Accessible name when no `<label for>` points here. */
  label?: string;
  onchange?: (value: T) => void;
}

let { id, value, options, disabled = false, label, onchange }: Props = $props();

function change(e: Event & { currentTarget: HTMLSelectElement }) {
  const picked = options[e.currentTarget.selectedIndex];
  if (picked) onchange?.(picked.value);
}
</script>

<select {id} {disabled} aria-label={label} onchange={change}>
  {#each options as option (option.value)}
    <option value={String(option.value)} selected={option.value === value}>{option.label}</option>
  {/each}
</select>

<style>
  select {
    font: var(--type-body);
    min-height: 28px;
    min-width: 160px;
    padding: 0 var(--space-2);
    border-radius: var(--radius-control);
    border: 1px solid var(--stroke);
    background: var(--surface-2);
    color: var(--text-1);
  }
  select:disabled {
    opacity: 0.5;
  }
  option {
    background: var(--surface-0-solid);
    color: var(--text-1);
  }
  @media (forced-colors: active) {
    select {
      border-color: ButtonText;
    }
  }
</style>

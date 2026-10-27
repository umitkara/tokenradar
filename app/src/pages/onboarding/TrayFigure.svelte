<!--
  A schematic of the Windows 11 notification area: the taskbar's right end with the ^ chevron,
  and the overflow flyout open above it with TokenRadar's ring among other icons. Drawn, not a
  screenshot: it follows the theme and high contrast, and carries no language.
-->
<script lang="ts">
let { label }: { label: string } = $props();

/** Other apps' icons in the flyout (3 × 2 grid, ours is the middle of the bottom row). */
const OTHERS: readonly { x: number; y: number }[] = [
  { x: 18, y: 14 },
  { x: 46, y: 14 },
  { x: 74, y: 14 },
  { x: 18, y: 42 },
  { x: 74, y: 42 },
];
</script>

<svg viewBox="0 0 320 150" role="img" aria-label={label}>
  <!-- The flyout. -->
  <rect class="panel" x="112" y="8" width="96" height="70" rx="8" />
  {#each OTHERS as { x, y } (`${x}:${y}`)}
    <rect class="other" x={112 + x - 8} y={8 + y + 2} width="16" height="16" rx="4" />
  {/each}
  <!-- TokenRadar's dual ring, highlighted. -->
  <rect class="focus" x="146" y="44" width="28" height="28" rx="6" />
  <circle class="ring-outer" cx="160" cy="58" r="9" />
  <circle class="ring-inner" cx="160" cy="58" r="5" />
  <!-- Pointer from the flyout down to the chevron. -->
  <path class="pointer" d="M160 80 L160 104" />

  <!-- The taskbar. -->
  <rect class="taskbar" x="0" y="112" width="320" height="38" />
  <rect class="other" x="24" y="123" width="16" height="16" rx="4" />
  <rect class="other" x="52" y="123" width="16" height="16" rx="4" />
  <!-- The ^ chevron, pressed. -->
  <rect class="focus" x="148" y="119" width="24" height="24" rx="4" />
  <path class="chevron" d="M154 134 L160 128 L166 134" />
  <!-- Clock. -->
  <rect class="text" x="262" y="122" width="40" height="6" rx="3" />
  <rect class="text" x="268" y="134" width="28" height="6" rx="3" />
</svg>

<style>
  svg {
    display: block;
    width: 100%;
    max-width: 320px;
    height: auto;
  }
  .panel,
  .taskbar {
    fill: var(--surface-2);
    stroke: var(--stroke);
  }
  /* Other apps' icons stay in the background; only ours is drawn in the accent. */
  .other {
    fill: var(--text-3);
    opacity: 0.45;
  }
  .text {
    fill: var(--text-2);
  }
  .focus {
    fill: var(--surface-1);
    stroke: var(--accent);
    stroke-width: 1.5;
  }
  .ring-outer,
  .ring-inner {
    fill: none;
    stroke: var(--accent);
  }
  .ring-outer {
    stroke-width: 2;
  }
  .ring-inner {
    stroke-width: 3;
  }
  .chevron,
  .pointer {
    fill: none;
    stroke: var(--text-1);
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .pointer {
    stroke-dasharray: 3 4;
    stroke: var(--text-2);
  }
  @media (forced-colors: active) {
    .panel,
    .taskbar {
      fill: Canvas;
      stroke: CanvasText;
    }
    .other,
    .text {
      fill: GrayText;
    }
    .focus {
      fill: Canvas;
      stroke: Highlight;
    }
    .ring-outer,
    .ring-inner {
      stroke: Highlight;
    }
    .chevron,
    .pointer {
      stroke: CanvasText;
    }
  }
</style>

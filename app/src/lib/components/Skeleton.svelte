<!-- A placeholder block while data loads. Static on purpose: no shimmer (ADR-019). -->
<script lang="ts">
let { width = "100%", height = "12px" }: { width?: string; height?: string } = $props();
</script>

<span class="skeleton" style:width style:height aria-hidden="true"></span>

<style>
  .skeleton {
    display: block;
    border-radius: var(--radius-control);
    background: var(--track);
    opacity: 0.6;
  }
</style>

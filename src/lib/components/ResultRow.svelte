<script lang="ts">
  import CopyButton from "./CopyButton.svelte";

  let {
    label,
    value,
    mono = true,
  }: { label: string; value: string; mono?: boolean } = $props();
</script>

<div
  class="flex items-center gap-3 border-b border-slate-100 py-2 last:border-0 dark:border-slate-800"
>
  <span class="w-32 shrink-0 text-xs text-slate-400">{label}</span>
  <span
    class="flex-1 select-text break-all text-sm text-slate-700 dark:text-slate-200 {mono
      ? 'font-mono'
      : ''}"
  >
    {value}
  </span>
  <CopyButton text={value} />
</div>

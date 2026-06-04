<script lang="ts">
  type Option = { label: string; value: string | number };
  let {
    options,
    value = $bindable(),
  }: { options: Option[]; value: string | number } = $props();
</script>

<div class="inline-flex rounded-lg bg-slate-100 p-0.5 dark:bg-slate-800">
  {#each options as o (o.value)}
    <button
      onclick={() => (value = o.value)}
      class="rounded-md px-3 py-1 text-sm font-medium transition {value === o.value
        ? 'bg-white text-indigo-600 shadow-sm dark:bg-slate-700 dark:text-indigo-300'
        : 'text-slate-500 hover:text-slate-700 dark:text-slate-400 dark:hover:text-slate-200'}"
    >
      {o.label}
    </button>
  {/each}
</div>

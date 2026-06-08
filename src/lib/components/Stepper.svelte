<script lang="ts">
  // 紧凑数字步进器：[−][value][+]，隐藏原生丑陋的上下箭头，可直接输入。
  let {
    value = $bindable(0),
    min = -Infinity,
    max = Infinity,
    step = 1,
    width = "w-12",
  }: {
    value?: number;
    min?: number;
    max?: number;
    step?: number;
    width?: string;
  } = $props();

  const clamp = (n: number) => Math.max(min, Math.min(max, n));
  const bump = (d: number) => (value = clamp((Number(value) || 0) + d));
</script>

<div class="inline-flex items-center overflow-hidden rounded-md border border-slate-200 dark:border-slate-700">
  <button
    type="button"
    class="px-2 py-0.5 text-sm text-slate-500 transition hover:bg-slate-100 hover:text-indigo-600 dark:hover:bg-slate-800 dark:hover:text-indigo-300"
    aria-label="减小"
    onclick={() => bump(-step)}
  >−</button>
  <input
    type="number"
    bind:value
    {min}
    {max}
    onchange={() => (value = clamp(Number(value) || 0))}
    class="{width} border-x border-slate-200 bg-transparent py-0.5 text-center text-xs text-slate-700 outline-none dark:border-slate-700 dark:text-slate-200 [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
  />
  <button
    type="button"
    class="px-2 py-0.5 text-sm text-slate-500 transition hover:bg-slate-100 hover:text-indigo-600 dark:hover:bg-slate-800 dark:hover:text-indigo-300"
    aria-label="增大"
    onclick={() => bump(step)}
  >+</button>
</div>

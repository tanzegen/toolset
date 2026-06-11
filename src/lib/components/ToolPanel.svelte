<script lang="ts">
  import type { Snippet } from "svelte";
  let {
    title,
    description,
    wide = false,
    children,
  }: { title: string; description?: string; wide?: boolean; children: Snippet } = $props();
</script>

<!-- wide：大文本框类工具占屏宽约 60%（60vw），上限 90rem 防超宽屏行太长；
     普通表单类工具仍用 max-w-4xl 保持紧凑易读。 -->
<div class="mx-auto {wide ? 'max-w-[min(60vw,90rem)]' : 'max-w-4xl'} px-8 py-7">
  <header class="mb-6">
    <h1 class="text-xl font-semibold text-slate-800 dark:text-slate-100">
      {title}
    </h1>
    {#if description}
      <p class="mt-1 text-sm text-slate-500 dark:text-slate-400">{description}</p>
    {/if}
  </header>
  {@render children()}
</div>

<script lang="ts">
  import type { Snippet } from "svelte";
  let {
    title,
    description,
    children,
  }: { title: string; description?: string; children: Snippet } = $props();
</script>

<div class="mx-auto max-w-4xl px-8 py-7">
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

<script lang="ts">
  import { tools } from "../tools";
  import { appState, toggleTheme } from "../state.svelte";
  import Icon from "./Icon.svelte";

  let query = $state("");

  const filtered = $derived(
    tools.filter(
      (t) =>
        t.name.toLowerCase().includes(query.toLowerCase()) ||
        t.desc.toLowerCase().includes(query.toLowerCase()),
    ),
  );

  // 按分类分组（保持注册表顺序）
  const groups = $derived.by(() => {
    const map = new Map<string, typeof tools>();
    for (const t of filtered) {
      if (!map.has(t.category)) map.set(t.category, []);
      map.get(t.category)!.push(t);
    }
    return [...map.entries()];
  });
</script>

<aside
  class="flex h-screen w-60 shrink-0 flex-col border-r border-slate-200 bg-slate-50/80 dark:border-slate-800 dark:bg-slate-900"
>
  <!-- 标题栏 -->
  <div class="flex items-center justify-between px-4 pb-2 pt-4">
    <div class="flex items-center gap-2">
      <div
        class="flex h-7 w-7 items-center justify-center rounded-lg bg-indigo-600 text-white"
      >
        <Icon name="chip" size={16} />
      </div>
      <span class="font-semibold text-slate-800 dark:text-slate-100">工具集</span>
    </div>
    <button
      onclick={toggleTheme}
      title="切换主题"
      aria-label="切换主题"
      class="rounded-md p-1.5 text-slate-400 transition hover:bg-slate-200 hover:text-slate-700 dark:hover:bg-slate-800 dark:hover:text-slate-200"
    >
      <Icon name={appState.theme === "dark" ? "sun" : "moon"} size={18} />
    </button>
  </div>

  <!-- 搜索 -->
  <div class="px-3 pb-2">
    <div class="relative">
      <span class="pointer-events-none absolute left-2.5 top-2 text-slate-400">
        <Icon name="search" size={16} />
      </span>
      <input
        bind:value={query}
        placeholder="搜索工具…"
        class="w-full rounded-lg border border-slate-200 bg-white py-1.5 pl-8 pr-2 text-sm outline-none transition focus:border-indigo-500 focus:ring-2 focus:ring-indigo-500/30 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
      />
    </div>
  </div>

  <!-- 工具列表 -->
  <nav class="flex-1 overflow-y-auto px-2 pb-3">
    {#each groups as [category, items] (category)}
      <div class="px-2 pb-1 pt-3 text-[11px] font-semibold uppercase tracking-wider text-slate-400">
        {category}
      </div>
      {#each items as t (t.id)}
        <button
          onclick={() => (appState.activeTool = t.id)}
          class="mb-0.5 flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left text-sm transition {appState.activeTool ===
          t.id
            ? 'bg-indigo-600 text-white shadow-sm'
            : 'text-slate-600 hover:bg-slate-200/60 dark:text-slate-300 dark:hover:bg-slate-800'}"
        >
          <Icon name={t.icon} size={17} />
          <span class="truncate">{t.name}</span>
        </button>
      {/each}
    {/each}
    {#if groups.length === 0}
      <p class="px-3 py-4 text-sm text-slate-400">无匹配工具</p>
    {/if}
  </nav>

  <div class="border-t border-slate-200 px-4 py-2 text-[11px] text-slate-400 dark:border-slate-800">
    本地运行 · 数据不出本机
  </div>
</aside>

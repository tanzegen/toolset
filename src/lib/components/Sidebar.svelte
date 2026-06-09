<script lang="ts">
  import { tools, type ToolDef } from "../tools";
  import { appState, toggleTheme, togglePin, toggleSidebar, setSidebarWidth, setActiveTool } from "../state.svelte";
  import { resizeHandle } from "../resize";
  import Icon from "./Icon.svelte";

  let query = $state("");
  let asideEl = $state<HTMLElement>();
  let resizing = $state(false);

  const filtered = $derived(
    tools.filter(
      (t) =>
        t.name.toLowerCase().includes(query.toLowerCase()) ||
        t.desc.toLowerCase().includes(query.toLowerCase()),
    ),
  );

  // 按分类分组（保持注册表顺序）
  const groups = $derived.by(() => {
    const map = new Map<string, ToolDef[]>();
    for (const t of filtered) {
      if (!map.has(t.category)) map.set(t.category, []);
      map.get(t.category)!.push(t);
    }
    return [...map.entries()];
  });

  const pinnedTools = $derived(
    appState.pinned
      .map((id) => tools.find((t) => t.id === id))
      .filter((t): t is ToolDef => !!t),
  );

  const collapsed = $derived(appState.sidebarCollapsed);
  const isPinned = (id: string) => appState.pinned.includes(id);
</script>

<aside
  bind:this={asideEl}
  class="relative flex h-screen shrink-0 flex-col border-r border-slate-200 bg-slate-50/80 dark:border-slate-800 dark:bg-slate-900 {collapsed
    ? 'w-14 transition-[width]'
    : ''} {resizing ? '' : 'transition-[width]'}"
  style={collapsed ? "" : `width:${appState.sidebarWidth}px`}
>
  <!-- 标题栏 -->
  {#if collapsed}
    <!-- 收起态：logo + 展开/主题按钮，与下方菜单图标同列同宽对齐，再用分隔带隔开 -->
    <div class="flex flex-col items-center gap-1 px-1.5 pt-3">
      <div class="mb-1 flex h-7 w-7 items-center justify-center rounded-lg bg-indigo-600 text-white">
        <Icon name="chip" size={16} />
      </div>
      <button
        onclick={toggleSidebar}
        title="展开侧栏"
        aria-label="展开侧栏"
        class="flex w-full items-center justify-center rounded-lg p-2 text-slate-400 transition hover:bg-slate-200/60 hover:text-slate-700 dark:hover:bg-slate-800 dark:hover:text-slate-200"
      >
        <Icon name="chevron-left" size={18} class="rotate-180" />
      </button>
      <button
        onclick={toggleTheme}
        title="切换主题"
        aria-label="切换主题"
        class="flex w-full items-center justify-center rounded-lg p-2 text-slate-400 transition hover:bg-slate-200/60 hover:text-slate-700 dark:hover:bg-slate-800 dark:hover:text-slate-200"
      >
        <Icon name={appState.theme === "dark" ? "sun" : "moon"} size={18} />
      </button>
    </div>
    <div class="mx-1.5 my-1.5 border-t border-slate-200 dark:border-slate-800"></div>
  {:else}
    <div class="flex items-center justify-between px-4 pb-2 pt-4">
      <div class="flex items-center gap-2">
        <div class="flex h-7 w-7 items-center justify-center rounded-lg bg-indigo-600 text-white">
          <Icon name="chip" size={16} />
        </div>
        <span class="font-semibold text-slate-800 dark:text-slate-100">工具集</span>
      </div>
      <div class="flex items-center gap-1">
        <button
          onclick={toggleSidebar}
          title="收起侧栏"
          aria-label="收起侧栏"
          class="rounded-md p-1.5 text-slate-400 transition hover:bg-slate-200 hover:text-slate-700 dark:hover:bg-slate-800 dark:hover:text-slate-200"
        >
          <Icon name="chevron-left" size={18} />
        </button>
        <button
          onclick={toggleTheme}
          title="切换主题"
          aria-label="切换主题"
          class="rounded-md p-1.5 text-slate-400 transition hover:bg-slate-200 hover:text-slate-700 dark:hover:bg-slate-800 dark:hover:text-slate-200"
        >
          <Icon name={appState.theme === "dark" ? "sun" : "moon"} size={18} />
        </button>
      </div>
    </div>
  {/if}

  {#if !collapsed}
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
  {/if}

  <!-- 列表 -->
  <nav class="flex-1 overflow-y-auto {collapsed ? 'px-1.5' : 'px-2'} pb-3">
    {#if collapsed}
      {#if pinnedTools.length}
        {#each pinnedTools as t (t.id)}{@render railIcon(t)}{/each}
        <div class="my-1.5 border-t border-slate-200 dark:border-slate-800"></div>
      {/if}
      {#each tools as t (t.id)}{@render railIcon(t)}{/each}
    {:else}
      {#if pinnedTools.length && !query}
        <div class="px-2 pb-1 pt-3 text-[11px] font-semibold uppercase tracking-wider text-amber-500">
          已固定
        </div>
        {#each pinnedTools as t (t.id)}{@render toolRow(t)}{/each}
      {/if}
      {#each groups as [category, items] (category)}
        <div class="px-2 pb-1 pt-3 text-[11px] font-semibold uppercase tracking-wider text-slate-400">
          {category}
        </div>
        {#each items as t (t.id)}{@render toolRow(t)}{/each}
      {/each}
      {#if groups.length === 0}
        <p class="px-3 py-4 text-sm text-slate-400">无匹配工具</p>
      {/if}
    {/if}
  </nav>

  {#if !collapsed}
    <div class="border-t border-slate-200 px-4 py-2 text-[11px] text-slate-400 dark:border-slate-800">
      本地运行 · 数据不出本机
    </div>
    <!-- 右缘拖拽改宽 -->
    <div
      role="separator"
      aria-orientation="vertical"
      title="拖动调整宽度"
      class="absolute right-0 top-0 z-10 h-full w-1.5 cursor-col-resize transition hover:bg-indigo-400/40 {resizing
        ? 'bg-indigo-400/50'
        : ''}"
      use:resizeHandle={{
        onstart: () => (resizing = true),
        onend: () => (resizing = false),
        onmove: (clientX) => {
          if (asideEl) setSidebarWidth(clientX - asideEl.getBoundingClientRect().left);
        },
      }}
    ></div>
  {/if}
</aside>

{#snippet railIcon(t: ToolDef)}
  <button
    onclick={() => setActiveTool(t.id)}
    title={t.name}
    aria-label={t.name}
    class="mb-0.5 flex w-full items-center justify-center rounded-lg p-2 transition {appState.activeTool ===
    t.id
      ? 'bg-indigo-600 text-white'
      : 'text-slate-500 hover:bg-slate-200/60 dark:text-slate-400 dark:hover:bg-slate-800'}"
  >
    <Icon name={t.icon} size={18} />
  </button>
{/snippet}

{#snippet toolRow(t: ToolDef)}
  {@const active = appState.activeTool === t.id}
  <div class="group relative mb-0.5">
    <button
      onclick={() => setActiveTool(t.id)}
      class="flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 pr-8 text-left text-sm transition {active
        ? 'bg-indigo-600 text-white shadow-sm'
        : 'text-slate-600 hover:bg-slate-200/60 dark:text-slate-300 dark:hover:bg-slate-800'}"
    >
      <Icon name={t.icon} size={17} />
      <span class="truncate">{t.name}</span>
    </button>
    <button
      onclick={() => togglePin(t.id)}
      title={isPinned(t.id) ? "取消固定" : "固定到顶部"}
      aria-label="固定"
      class="absolute right-1.5 top-1/2 -translate-y-1/2 rounded p-1 transition hover:text-amber-400 {isPinned(
        t.id,
      )
        ? 'text-amber-400 opacity-100'
        : 'text-slate-400 opacity-0 group-hover:opacity-100'} {active ? 'text-white' : ''}"
    >
      <Icon name="pin" size={13} />
    </button>
  </div>
{/snippet}

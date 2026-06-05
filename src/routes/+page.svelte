<script lang="ts">
  import { untrack } from "svelte";
  import Sidebar from "../lib/components/Sidebar.svelte";
  import { appState } from "../lib/state.svelte";
  import { tools } from "../lib/tools";

  // keep-alive：工具首次访问时挂载，之后切换只切显隐而不销毁，
  // 从而保留各工具已输入/已生成的内容，切回来仍在。
  let mounted = $state<string[]>([]);
  $effect(() => {
    const id = appState.activeTool;
    untrack(() => {
      if (!mounted.includes(id)) mounted.push(id);
    });
  });
</script>

<div
  class="flex h-screen overflow-hidden bg-white text-slate-900 dark:bg-slate-950 dark:text-slate-100"
>
  <Sidebar />
  <main class="flex-1 overflow-y-auto">
    {#each tools as tool (tool.id)}
      {#if mounted.includes(tool.id)}
        {@const Tool = tool.component}
        <div class:hidden={tool.id !== appState.activeTool}>
          <Tool />
        </div>
      {/if}
    {/each}
  </main>
</div>

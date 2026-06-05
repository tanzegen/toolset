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

<!-- 全局兜底：拦截文件拖放，避免拖到无处理区时 WebView 直接打开文件（像“开了个新窗口”）。
     终端与 SFTP 面板各自有自己的 drop 处理，会先于此处运行。 -->
<div
  class="flex h-screen overflow-hidden bg-white text-slate-900 dark:bg-slate-950 dark:text-slate-100"
  role="presentation"
  ondragover={(e) => e.preventDefault()}
  ondrop={(e) => e.preventDefault()}
>
  <Sidebar />
  <!-- 列向 flex：可见工具填满高度，各自内部滚动（终端类工具需要 h-full 生效）。 -->
  <main class="flex min-w-0 flex-1 flex-col overflow-hidden">
    {#each tools as tool (tool.id)}
      {#if mounted.includes(tool.id)}
        {@const Tool = tool.component}
        <div class="min-h-0 flex-1 overflow-y-auto" class:hidden={tool.id !== appState.activeTool}>
          <Tool />
        </div>
      {/if}
    {/each}
  </main>
</div>

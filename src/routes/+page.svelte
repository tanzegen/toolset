<script lang="ts">
  import Sidebar from "../lib/components/Sidebar.svelte";
  import { appState } from "../lib/state.svelte";
  import { findTool } from "../lib/tools";

  const active = $derived(findTool(appState.activeTool));
</script>

<div
  class="flex h-screen overflow-hidden bg-white text-slate-900 dark:bg-slate-950 dark:text-slate-100"
>
  <Sidebar />
  <main class="flex-1 overflow-y-auto">
    {#key active.id}
      {@const Tool = active.component}
      <Tool />
    {/key}
  </main>
</div>

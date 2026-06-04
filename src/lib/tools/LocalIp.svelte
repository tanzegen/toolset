<script lang="ts">
  import { onMount } from "svelte";
  import { api, errMsg, type LocalIpResult } from "../ipc";
  import ToolPanel from "../components/ToolPanel.svelte";
  import CopyButton from "../components/CopyButton.svelte";
  import { cls } from "../ui";

  let result = $state<LocalIpResult | null>(null);
  let error = $state("");
  let loading = $state(false);

  async function load() {
    loading = true;
    error = "";
    try {
      result = await api.localIps();
    } catch (e) {
      error = errMsg(e);
      result = null;
    } finally {
      loading = false;
    }
  }

  onMount(load);
</script>

<ToolPanel title="内网 IP" description="本机各网卡地址（纯本地枚举，不联网）。">
  <div class="{cls.card} flex flex-wrap items-center gap-3 px-4 py-4">
    <span class="w-20 shrink-0 text-xs text-slate-400">主用出口</span>
    <span class="flex-1 select-text break-all font-mono text-lg text-indigo-600 dark:text-indigo-300">
      {loading ? "读取中…" : (result?.primary ?? "—")}
    </span>
    {#if result?.primary}<CopyButton text={result.primary} />{/if}
    <button class={cls.btn} onclick={load} disabled={loading}>刷新</button>
  </div>

  {#if error}
    <p class="mt-4 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-600 dark:bg-red-950/40 dark:text-red-400">
      {error}
    </p>
  {/if}

  {#if result && result.interfaces.length}
    <div class="{cls.card} mt-3 px-4 py-2">
      {#each result.interfaces as i, idx (idx)}
        <div class="flex items-center gap-3 border-b border-slate-100 py-2 last:border-0 dark:border-slate-800">
          <span class="w-44 shrink-0 truncate text-xs text-slate-400" title={i.name}>{i.name}</span>
          <span class="shrink-0 rounded bg-slate-100 px-1.5 py-0.5 text-[10px] text-slate-500 dark:bg-slate-800 dark:text-slate-400">IPv{i.version}</span>
          {#if i.is_loopback}
            <span class="shrink-0 rounded bg-slate-100 px-1.5 py-0.5 text-[10px] text-slate-400 dark:bg-slate-800">环回</span>
          {/if}
          <span class="flex-1 select-text break-all font-mono text-sm text-slate-700 dark:text-slate-200">{i.ip}</span>
          <CopyButton text={i.ip} />
        </div>
      {/each}
    </div>
  {/if}
</ToolPanel>

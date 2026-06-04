<script lang="ts">
  import { onMount } from "svelte";
  import ToolPanel from "../components/ToolPanel.svelte";
  import CopyButton from "../components/CopyButton.svelte";
  import ComboBox from "../components/ComboBox.svelte";
  import { cls } from "../ui";

  // 预设端点（均已验证支持 CORS，可在 webview 内使用）；首项为默认。
  const IP_ENDPOINTS = [
    "https://api.ipify.org",
    "https://api.ip.sb/ip",
    "https://ifconfig.me/ip",
    "https://icanhazip.com",
    "https://ipinfo.io/ip",
  ];

  let endpoint = $state(IP_ENDPOINTS[0]);
  let ip = $state("");
  let err = $state("");
  let loading = $state(false);

  onMount(() => {
    const a = localStorage.getItem("net-ip-endpoint");
    if (a) endpoint = a;
    query();
  });

  $effect(() => {
    localStorage.setItem("net-ip-endpoint", endpoint);
  });

  async function query() {
    loading = true;
    err = "";
    ip = "";
    try {
      const res = await fetch(endpoint);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const text = (await res.text()).trim();
      try {
        const j = JSON.parse(text);
        ip = j.ip ?? text;
      } catch {
        ip = text;
      }
    } catch (e) {
      err = `查询失败：${e instanceof Error ? e.message : String(e)}（可能被 CORS / 网络拦截，试试换端点）`;
    } finally {
      loading = false;
    }
  }
</script>

<ToolPanel
  title="公网 IP"
  description="⚠ 需联网：向第三方回显服务查询当前公网出口 IP。端点可自定义。"
>
  <div class="{cls.card} px-4 py-4">
    <div class="flex items-center gap-3">
      <span class="w-20 shrink-0 text-xs text-slate-400">当前公网 IP</span>
      <span class="flex-1 select-text break-all font-mono text-lg text-indigo-600 dark:text-indigo-300">
        {loading ? "查询中…" : ip || "—"}
      </span>
      {#if ip}<CopyButton text={ip} />{/if}
      <button class={cls.btn} onclick={query} disabled={loading}>刷新</button>
    </div>
    {#if err}
      <p class="mt-2 text-sm text-red-600 dark:text-red-400">{err}</p>
    {/if}
  </div>

  <div class="mt-3 flex items-center gap-2">
    <span class="{cls.label} w-16 shrink-0">端点</span>
    <ComboBox bind:value={endpoint} options={IP_ENDPOINTS} placeholder="回显服务端点（可自定义）" />
    <button class="{cls.btn} shrink-0" onclick={() => (endpoint = IP_ENDPOINTS[0])} title="恢复默认">默认</button>
  </div>
  <p class="mt-1 text-xs text-slate-400">点击端点框可展开预设，或自行输入；webview 内只能用支持 CORS 的服务。</p>
</ToolPanel>

<script lang="ts">
  import { onMount } from "svelte";
  import ToolPanel from "../components/ToolPanel.svelte";
  import SegmentedControl from "../components/SegmentedControl.svelte";
  import CopyButton from "../components/CopyButton.svelte";
  import ComboBox from "../components/ComboBox.svelte";
  import { cls } from "../ui";

  // 预设 DoH 端点（均已验证支持 CORS）；首项为默认（阿里，国内快）。
  const DOH_ENDPOINTS = [
    "https://dns.alidns.com/resolve",
    "https://cloudflare-dns.com/dns-query",
    "https://dns.google/resolve",
  ];

  interface DohAnswer {
    name: string;
    type: number;
    TTL: number;
    data: string;
  }
  const TYPE_MAP: Record<number, string> = {
    1: "A",
    2: "NS",
    5: "CNAME",
    6: "SOA",
    12: "PTR",
    15: "MX",
    16: "TXT",
    28: "AAAA",
  };

  let endpoint = $state(DOH_ENDPOINTS[0]);
  let domain = $state("example.com");
  let recordType = $state("A");
  let records = $state<{ type: string; data: string; ttl: number }[]>([]);
  let err = $state("");
  let loading = $state(false);

  onMount(() => {
    const b = localStorage.getItem("net-doh-endpoint");
    if (b) endpoint = b;
  });

  $effect(() => {
    localStorage.setItem("net-doh-endpoint", endpoint);
  });

  async function query() {
    if (!domain.trim()) return;
    loading = true;
    err = "";
    records = [];
    try {
      const url = `${endpoint}?name=${encodeURIComponent(domain.trim())}&type=${recordType}`;
      const res = await fetch(url, { headers: { Accept: "application/dns-json" } });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const j = (await res.json()) as { Status?: number; Answer?: DohAnswer[] };
      const ans = j.Answer ?? [];
      records = ans.map((a) => ({
        type: TYPE_MAP[a.type] ?? String(a.type),
        data: a.data,
        ttl: a.TTL,
      }));
      if (records.length === 0) err = `无 ${recordType} 记录（Status=${j.Status ?? "?"}）`;
    } catch (e) {
      err = `查询失败：${e instanceof Error ? e.message : String(e)}（试试换 DoH 端点）`;
    } finally {
      loading = false;
    }
  }
</script>

<ToolPanel
  title="域名解析"
  description="⚠ 需联网：通过 DoH (DNS over HTTPS) 查询域名记录。端点可自定义。"
>
  <div class="flex flex-wrap items-center gap-3">
    <input
      bind:value={domain}
      onkeydown={(e) => e.key === "Enter" && query()}
      placeholder="输入域名，如 example.com"
      class="{cls.field} flex-1 font-mono"
    />
    <SegmentedControl
      bind:value={recordType}
      options={[
        { label: "A", value: "A" },
        { label: "AAAA", value: "AAAA" },
        { label: "CNAME", value: "CNAME" },
        { label: "MX", value: "MX" },
        { label: "TXT", value: "TXT" },
        { label: "NS", value: "NS" },
      ]}
    />
    <button class={cls.btnPrimary} onclick={query} disabled={loading}>
      {loading ? "查询中…" : "查询"}
    </button>
  </div>

  <div class="mt-3 flex items-center gap-2">
    <span class="{cls.label} w-16 shrink-0">DoH</span>
    <ComboBox bind:value={endpoint} options={DOH_ENDPOINTS} placeholder="DoH JSON 端点（可自定义）" />
    <button class="{cls.btn} shrink-0" onclick={() => (endpoint = DOH_ENDPOINTS[0])} title="恢复默认">默认</button>
  </div>
  <p class="mt-1 text-xs text-slate-400">点击 DoH 框可展开预设，或自行输入；需为支持 CORS 的 DoH JSON 接口。</p>

  {#if err}
    <p class="mt-4 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-600 dark:bg-red-950/40 dark:text-red-400">
      {err}
    </p>
  {/if}

  {#if records.length}
    <div class="{cls.card} mt-4 px-4 py-2">
      {#each records as r, i (i)}
        <div class="flex items-center gap-3 border-b border-slate-100 py-2 last:border-0 dark:border-slate-800">
          <span class="w-14 shrink-0 text-xs font-medium text-indigo-600 dark:text-indigo-300">{r.type}</span>
          <span class="flex-1 select-text break-all font-mono text-sm text-slate-700 dark:text-slate-200">{r.data}</span>
          <span class="shrink-0 text-xs text-slate-400">TTL {r.ttl}s</span>
          <CopyButton text={r.data} />
        </div>
      {/each}
    </div>
  {/if}
</ToolPanel>

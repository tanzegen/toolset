<script lang="ts">
  import { api, errMsg, type SubnetResult } from "../ipc";
  import ToolPanel from "../components/ToolPanel.svelte";
  import ResultRow from "../components/ResultRow.svelte";
  import { cls } from "../ui";

  let input = $state("192.168.1.10/24");
  let result = $state<SubnetResult | null>(null);
  let error = $state("");
  let seq = 0;

  async function run() {
    const id = ++seq;
    error = "";
    if (!input.trim()) {
      result = null;
      return;
    }
    try {
      const r = await api.subnetCalc(input);
      if (id === seq) result = r;
    } catch (e) {
      if (id === seq) {
        error = errMsg(e);
        result = null;
      }
    }
  }

  $effect(() => {
    void input;
    run();
  });
</script>

<ToolPanel
  title="子网计算器"
  description="解析 CIDR 或 IP+掩码，给出网络、广播、可用范围与掩码。支持 IPv4 / IPv6。"
>
  <input
    bind:value={input}
    placeholder="如 192.168.1.10/24、10.0.0.5 255.255.255.0、2001:db8::1/64"
    class="{cls.field} font-mono"
  />

  {#if error}
    <p class="mt-4 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-600 dark:bg-red-950/40 dark:text-red-400">
      {error}
    </p>
  {/if}

  {#if result}
    <div class="{cls.card} mt-5 px-4 py-2">
      <ResultRow label="版本" value={`IPv${result.version}`} mono={false} />
      <ResultRow label="CIDR" value={result.cidr} />
      <ResultRow label="网络地址" value={result.network} />
      <ResultRow label={result.version === 4 ? "广播地址" : "末地址"} value={result.broadcast} />
      <ResultRow label="子网掩码" value={`${result.netmask}  (/${result.prefix})`} />
      <ResultRow label="通配符掩码" value={result.wildcard} />
      <ResultRow label="可用主机范围" value={`${result.first_host} – ${result.last_host}`} />
    </div>
    <div class="{cls.card} mt-3 px-4 py-2">
      <ResultRow label="可用主机数" value={result.usable} />
      <ResultRow label="总地址数" value={result.total} />
      <ResultRow label="网络号(十进制)" value={result.network_int} />
      <ResultRow label="地址类别" value={result.ip_class} mono={false} />
      <ResultRow label="私有地址" value={result.is_private ? "是" : "否"} mono={false} />
    </div>
  {/if}
</ToolPanel>

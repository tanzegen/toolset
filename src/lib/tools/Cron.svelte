<script lang="ts">
  import { onMount } from "svelte";
  import { api, errMsg, type CronResult } from "../ipc";
  import ToolPanel from "../components/ToolPanel.svelte";
  import TzSelect from "../components/TzSelect.svelte";
  import { cls } from "../ui";

  let expr = $state("0 9 * * 1-5");
  let tz = $state("UTC");
  let count = $state(5);
  let result = $state<CronResult | null>(null);
  let error = $state("");
  let seq = 0;

  onMount(() => {
    try {
      tz = Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
    } catch {}
  });

  async function run() {
    const id = ++seq;
    error = "";
    if (!expr.trim()) {
      result = null;
      return;
    }
    try {
      const r = await api.cronExplain(expr, tz, count || 5);
      if (id === seq) result = r;
    } catch (e) {
      if (id === seq) {
        error = errMsg(e);
        result = null;
      }
    }
  }

  $effect(() => {
    void [expr, tz, count];
    run();
  });
</script>

<ToolPanel title="Cron 表达式" description="解析 5/6 字段 cron，预测未来运行时间（按时区）。">
  <div class="space-y-3">
    <input
      bind:value={expr}
      placeholder="如 0 9 * * 1-5（工作日 9 点）"
      class="{cls.field} font-mono"
    />
    <div class="flex flex-wrap items-center gap-3">
      <span class={cls.label}>时区</span>
      <div class="min-w-52 flex-1"><TzSelect bind:value={tz} /></div>
      <span class={cls.label}>条数</span>
      <input type="number" min="1" max="20" bind:value={count} class="{cls.field} w-20 font-mono" />
    </div>
  </div>

  {#if error}
    <p class="mt-4 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-600 dark:bg-red-950/40 dark:text-red-400">
      {error}
    </p>
  {/if}

  {#if result}
    <div class="{cls.card} mt-5 p-4">
      <div class="flex flex-wrap gap-2">
        {#each result.fields as f (f.label)}
          <div class="rounded-lg bg-slate-100 px-3 py-1.5 text-sm dark:bg-slate-800">
            <span class="text-slate-400">{f.label}</span>
            <span class="ml-1 font-mono text-indigo-600 dark:text-indigo-300">{f.value}</span>
          </div>
        {/each}
      </div>
    </div>

    <div class="{cls.card} mt-3 px-4 py-2">
      {#each result.next_runs as r, i (i)}
        <div class="flex items-center gap-3 border-b border-slate-100 py-2 last:border-0 dark:border-slate-800">
          <span class="w-6 text-xs text-slate-400">{i + 1}</span>
          <span class="flex-1 font-mono text-sm text-slate-700 dark:text-slate-200">{r.local}</span>
          <span class="text-xs text-slate-400">{r.relative}</span>
        </div>
      {/each}
      {#if result.next_runs.length === 0}
        <p class="py-2 text-sm text-slate-400">可见范围内无运行时间</p>
      {/if}
    </div>
    <p class="mt-2 text-xs text-slate-400">时区 {result.timezone} · 右侧为相对现在</p>
  {/if}
</ToolPanel>

<script lang="ts">
  import { onMount } from "svelte";
  import { api, errMsg, type TimestampResult } from "../ipc";
  import ToolPanel from "../components/ToolPanel.svelte";
  import SegmentedControl from "../components/SegmentedControl.svelte";
  import ResultRow from "../components/ResultRow.svelte";
  import TzSelect from "../components/TzSelect.svelte";
  import { cls } from "../ui";

  let mode = $state<string>("epoch");
  let input = $state("");
  let unit = $state<string>("auto");
  let datetime = $state("");
  let tz = $state("UTC"); // 已提交、驱动换算的时区
  let result = $state<TimestampResult | null>(null);
  let error = $state("");
  let seq = 0;

  const unitLabel: Record<string, string> = {
    s: "秒 (s)",
    ms: "毫秒 (ms)",
    us: "微秒 (µs)",
    ns: "纳秒 (ns)",
    datetime: "日期时间",
  };

  onMount(async () => {
    try {
      tz = Intl.DateTimeFormat().resolvedOptions().timeZone || "UTC";
    } catch {}
    input = await api.nowMillis();
  });

  async function run() {
    const id = ++seq;
    error = "";
    try {
      let r: TimestampResult;
      if (mode === "epoch") {
        if (!input.trim()) {
          result = null;
          return;
        }
        r = await api.timestampConvert(input, unit, tz);
      } else {
        if (!datetime.trim()) {
          result = null;
          return;
        }
        r = await api.timestampFromDatetime(datetime, tz);
      }
      if (id === seq) result = r;
    } catch (e) {
      if (id === seq) {
        error = errMsg(e);
        result = null;
      }
    }
  }

  $effect(() => {
    void [mode, input, unit, datetime, tz];
    run();
  });

  async function fillNow() {
    mode = "epoch";
    unit = "auto";
    input = await api.nowMillis();
  }
</script>

<ToolPanel
  title="时间戳转换"
  description="动态识别 秒 / 毫秒 / 微秒 / 纳秒，按 IANA 时区双向换算。"
>
  <div class="mb-4 flex flex-wrap items-center gap-3">
    <SegmentedControl
      bind:value={mode}
      options={[
        { label: "时间戳 → 时间", value: "epoch" },
        { label: "时间 → 时间戳", value: "datetime" },
      ]}
    />
  </div>

  <div class="space-y-3">
    {#if mode === "epoch"}
      <div class="flex flex-wrap items-center gap-3">
        <input
          bind:value={input}
          placeholder="输入时间戳，如 1700000000"
          class="{cls.field} flex-1 font-mono"
        />
        <button class={cls.btn} onclick={fillNow}>现在</button>
      </div>
      <div class="flex flex-wrap items-center gap-3">
        <span class={cls.label}>单位</span>
        <SegmentedControl
          bind:value={unit}
          options={[
            { label: "自动", value: "auto" },
            { label: "秒", value: "s" },
            { label: "毫秒", value: "ms" },
            { label: "微秒", value: "us" },
            { label: "纳秒", value: "ns" },
          ]}
        />
      </div>
    {:else}
      <input
        bind:value={datetime}
        placeholder="输入本地日期时间，如 2023-11-15 06:13:20"
        class="{cls.field} font-mono"
      />
    {/if}

    <div class="flex items-center gap-3">
      <span class="{cls.label} w-10">时区</span>
      <TzSelect bind:value={tz} />
    </div>
  </div>

  {#if error}
    <p class="mt-4 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-600 dark:bg-red-950/40 dark:text-red-400">
      {error}
    </p>
  {/if}

  {#if result}
    <div class="{cls.card} mt-5 px-4 py-2">
      <ResultRow label="识别单位" value={unitLabel[result.detected_unit] ?? result.detected_unit} mono={false} />
      <ResultRow label={`本地时间 · ${result.timezone}`} value={result.local} />
      <ResultRow label="UTC" value={result.utc} />
      <ResultRow label="ISO 8601" value={result.iso8601} />
      <ResultRow label="RFC 2822" value={result.rfc2822} />
      <ResultRow label="星期 / 相对" value={`${result.weekday} · ${result.relative}`} mono={false} />
    </div>
    <div class="{cls.card} mt-3 px-4 py-2">
      <ResultRow label="秒 (s)" value={result.epoch_seconds} />
      <ResultRow label="毫秒 (ms)" value={result.epoch_millis} />
      <ResultRow label="微秒 (µs)" value={result.epoch_micros} />
      <ResultRow label="纳秒 (ns)" value={result.epoch_nanos} />
    </div>
  {/if}
</ToolPanel>

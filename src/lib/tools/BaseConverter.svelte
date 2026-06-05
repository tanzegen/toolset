<script lang="ts">
  import { api, errMsg, type BaseResult } from "../ipc";
  import ToolPanel from "../components/ToolPanel.svelte";
  import SegmentedControl from "../components/SegmentedControl.svelte";
  import ResultRow from "../components/ResultRow.svelte";
  import { cls } from "../ui";
  import { persist } from "../persist.svelte";

  let input = $state("255");
  let fromBase = $state<number>(10);
  let bitWidth = $state<number>(32);
  let result = $state<BaseResult | null>(null);
  let error = $state("");
  let seq = 0;

  persist("base", {
    save: () => ({ input, fromBase, bitWidth }),
    load: (s) => {
      input = s.input ?? input;
      fromBase = s.fromBase ?? fromBase;
      bitWidth = s.bitWidth ?? bitWidth;
    },
  });

  async function run() {
    const id = ++seq;
    error = "";
    if (!input.trim()) {
      result = null;
      return;
    }
    try {
      const r = await api.baseConvert(input, fromBase, bitWidth);
      if (id === seq) result = r;
    } catch (e) {
      if (id === seq) {
        error = errMsg(e);
        result = null;
      }
    }
  }

  $effect(() => {
    void [input, fromBase, bitWidth];
    run();
  });
</script>

<ToolPanel
  title="进制转换"
  description="二 / 八 / 十 / 十六进制互转，按位宽给出有符号与无符号解释。"
>
  <div class="space-y-3">
    <input
      bind:value={input}
      placeholder="输入数值，可带 0x / 0b / 0o 前缀，十进制可为负"
      class="{cls.field} font-mono"
    />
    <div class="flex flex-wrap items-center gap-3">
      <span class="{cls.label} w-10">输入进制</span>
      <SegmentedControl
        bind:value={fromBase}
        options={[
          { label: "二进制", value: 2 },
          { label: "八进制", value: 8 },
          { label: "十进制", value: 10 },
          { label: "十六进制", value: 16 },
        ]}
      />
    </div>
    <div class="flex flex-wrap items-center gap-3">
      <span class="{cls.label} w-10">位宽</span>
      <SegmentedControl
        bind:value={bitWidth}
        options={[
          { label: "8", value: 8 },
          { label: "16", value: 16 },
          { label: "32", value: 32 },
          { label: "64", value: 64 },
        ]}
      />
    </div>
  </div>

  {#if error}
    <p class="mt-4 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-600 dark:bg-red-950/40 dark:text-red-400">
      {error}
    </p>
  {/if}

  {#if result}
    <div class="{cls.card} mt-5 px-4 py-2">
      <ResultRow label="十六进制" value={`0x${result.hex}`} />
      <ResultRow label="十进制 (无符号)" value={result.dec_unsigned} />
      <ResultRow label={`十进制 (有符号 i${result.bit_width})`} value={result.dec_signed} />
      <ResultRow label="八进制" value={`0o${result.oct}`} />
      <ResultRow label="二进制" value={result.bin} />
    </div>
    <div class="{cls.card} mt-3 px-4 py-3">
      <div class="mb-1 text-xs text-slate-400">位视图 ({result.bit_width} 位)</div>
      <div class="select-text break-all font-mono text-sm text-indigo-600 dark:text-indigo-300">
        {result.bits_grouped}
      </div>
    </div>
  {/if}
</ToolPanel>

<script lang="ts">
  import { api, errMsg, type FloatResult } from "../ipc";
  import ToolPanel from "../components/ToolPanel.svelte";
  import SegmentedControl from "../components/SegmentedControl.svelte";
  import ResultRow from "../components/ResultRow.svelte";
  import { cls } from "../ui";
  import { persist } from "../persist.svelte";

  let mode = $state("hex2float");
  let width = $state("auto");
  let input = $state("40490FDB");
  let result = $state<FloatResult | null>(null);
  let error = $state("");
  let seq = 0;

  persist("hexfloat", {
    save: () => ({ mode, width, input }),
    load: (s) => {
      mode = s.mode ?? mode;
      width = s.width ?? width;
      input = s.input ?? input;
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
      const r =
        mode === "hex2float"
          ? await api.hexToFloat(input, width)
          : await api.floatToHex(input, width === "auto" ? "64" : width);
      if (id === seq) result = r;
    } catch (e) {
      if (id === seq) {
        error = errMsg(e);
        result = null;
      }
    }
  }

  $effect(() => {
    void [mode, width, input];
    run();
  });
</script>

<ToolPanel
  title="Hex / Float 转换"
  description="按 IEEE 754 解释十六进制位模式，或把浮点数转回位模式（32 / 64 位）。"
>
  <div class="mb-4 flex flex-wrap items-center gap-3">
    <SegmentedControl
      bind:value={mode}
      options={[
        { label: "Hex → Float", value: "hex2float" },
        { label: "Float → Hex", value: "float2hex" },
      ]}
    />
    <SegmentedControl
      bind:value={width}
      options={mode === "hex2float"
        ? [
            { label: "自动", value: "auto" },
            { label: "32 位", value: "32" },
            { label: "64 位", value: "64" },
          ]
        : [
            { label: "32 位", value: "32" },
            { label: "64 位", value: "64" },
          ]}
    />
  </div>

  <input
    bind:value={input}
    placeholder={mode === "hex2float" ? "输入十六进制，如 0x40490FDB" : "输入浮点数，如 3.1415927"}
    class="{cls.field} font-mono"
  />

  {#if error}
    <p class="mt-4 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-600 dark:bg-red-950/40 dark:text-red-400">
      {error}
    </p>
  {/if}

  {#if result}
    <div class="{cls.card} mt-5 px-4 py-2">
      <ResultRow label={`浮点值 (f${result.width})`} value={result.float_value} />
      <ResultRow label="十六进制位" value={result.hex} />
      <ResultRow label="二进制 (符号·阶码·尾数)" value={result.binary} />
    </div>
    <div class="{cls.card} mt-3 px-4 py-2">
      <ResultRow label="符号位" value={String(result.sign)} />
      <ResultRow label="阶码 (原始 / 去偏)" value={`${result.exponent_raw} / ${result.exponent_unbiased}`} />
      <ResultRow label="尾数" value={result.mantissa_hex} />
      <ResultRow label="类别" value={result.category} mono={false} />
    </div>
    <div class="{cls.card} mt-3 px-4 py-2">
      <ResultRow label="整数解释 (无符号)" value={result.int_unsigned} />
      <ResultRow label="整数解释 (有符号)" value={result.int_signed} />
    </div>
  {/if}
</ToolPanel>

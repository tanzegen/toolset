<script lang="ts">
  import { api, errMsg } from "../ipc";
  import ToolPanel from "../components/ToolPanel.svelte";
  import SegmentedControl from "../components/SegmentedControl.svelte";
  import CopyButton from "../components/CopyButton.svelte";
  import { cls } from "../ui";

  let action = $state("format");
  let indent = $state<number>(2);
  let input = $state('{"name":"工具集","tags":["dev","tools"],"n":42}');
  let output = $state("");
  let error = $state("");
  let seq = 0;

  async function run() {
    const id = ++seq;
    error = "";
    if (!input.trim()) {
      output = "";
      return;
    }
    try {
      const r =
        action === "format"
          ? await api.jsonFormat(input, indent)
          : await api.jsonMinify(input);
      if (id === seq) output = r;
    } catch (e) {
      if (id === seq) {
        error = errMsg(e);
        output = "";
      }
    }
  }

  $effect(() => {
    void [action, indent, input];
    run();
  });
</script>

<ToolPanel title="JSON 工具" description="格式化、压缩与校验，错误带行列定位。">
  <div class="mb-3 flex flex-wrap items-center gap-3">
    <SegmentedControl
      bind:value={action}
      options={[
        { label: "格式化", value: "format" },
        { label: "压缩", value: "minify" },
      ]}
    />
    {#if action === "format"}
      <SegmentedControl
        bind:value={indent}
        options={[
          { label: "2 空格", value: 2 },
          { label: "4 空格", value: 4 },
        ]}
      />
    {/if}
    {#if error}
      <span class="rounded-md bg-red-50 px-2 py-1 text-xs text-red-600 dark:bg-red-950/40 dark:text-red-400">
        {error}
      </span>
    {:else if output}
      <span class="rounded-md bg-emerald-50 px-2 py-1 text-xs text-emerald-600 dark:bg-emerald-950/40 dark:text-emerald-400">
        ✓ 合法 JSON
      </span>
    {/if}
  </div>

  <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
    <div>
      <div class="{cls.label} mb-1">输入</div>
      <textarea
        bind:value={input}
        spellcheck="false"
        class="{cls.field} h-80 resize-none font-mono leading-relaxed"
      ></textarea>
    </div>
    <div>
      <div class="mb-1 flex items-center justify-between">
        <span class={cls.label}>输出</span>
        {#if output}<CopyButton text={output} />{/if}
      </div>
      <textarea
        value={output}
        readonly
        spellcheck="false"
        class="{cls.field} h-80 resize-none bg-slate-50 font-mono leading-relaxed dark:bg-slate-900/60"
      ></textarea>
    </div>
  </div>
</ToolPanel>

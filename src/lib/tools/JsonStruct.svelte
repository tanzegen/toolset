<script lang="ts">
  import { api, errMsg } from "../ipc";
  import ToolPanel from "../components/ToolPanel.svelte";
  import SegmentedControl from "../components/SegmentedControl.svelte";
  import CopyButton from "../components/CopyButton.svelte";
  import { cls } from "../ui";

  let lang = $state("go");
  let rootName = $state("Root");
  let json = $state('{"id":1,"name":"工具集","tags":["dev"],"owner":{"id":2,"email":"a@b.c"}}');
  let output = $state("");
  let error = $state("");
  let seq = 0;

  async function run() {
    const id = ++seq;
    error = "";
    if (!json.trim()) {
      output = "";
      return;
    }
    try {
      const r = await api.jsonToStruct(json, lang, rootName || "Root");
      if (id === seq) output = r;
    } catch (e) {
      if (id === seq) {
        error = errMsg(e);
        output = "";
      }
    }
  }

  $effect(() => {
    void [json, lang, rootName];
    run();
  });
</script>

<ToolPanel title="JSON 转结构" description="按 JSON 推断类型，生成 Go / TypeScript / Rust 结构定义。">
  <div class="mb-3 flex flex-wrap items-center gap-3">
    <SegmentedControl
      bind:value={lang}
      options={[
        { label: "Go", value: "go" },
        { label: "TypeScript", value: "ts" },
        { label: "Rust", value: "rust" },
      ]}
    />
    <div class="flex shrink-0 items-center gap-2">
      <span class="{cls.label} whitespace-nowrap">根类型名</span>
      <input bind:value={rootName} class="{cls.field} w-40 font-mono" />
    </div>
  </div>

  {#if error}
    <p class="mb-3 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-600 dark:bg-red-950/40 dark:text-red-400">
      {error}
    </p>
  {/if}

  <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
    <div>
      <div class="mb-1 flex h-7 items-center"><span class={cls.label}>JSON 输入</span></div>
      <textarea
        bind:value={json}
        spellcheck="false"
        class="{cls.field} h-80 resize-none font-mono leading-relaxed"
      ></textarea>
    </div>
    <div>
      <div class="mb-1 flex h-7 items-center justify-between">
        <span class={cls.label}>结构输出</span>
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

<script lang="ts">
  import { api, errMsg } from "../ipc";
  import ToolPanel from "../components/ToolPanel.svelte";
  import SegmentedControl from "../components/SegmentedControl.svelte";
  import CopyButton from "../components/CopyButton.svelte";
  import { cls } from "../ui";
  import { persist } from "../persist.svelte";

  let target = $state("hant");
  let input = $state("简繁转换：电脑、软件、网络、内存、里程、皇后");
  let output = $state("");
  let error = $state("");
  let seq = 0;

  persist("zhconvert", {
    save: () => ({ target, input }),
    load: (s) => {
      target = s.target ?? target;
      input = s.input ?? input;
    },
  });

  async function run() {
    const id = ++seq;
    error = "";
    if (!input.trim()) {
      output = "";
      return;
    }
    try {
      const r = await api.zhConvert(input, target);
      if (id === seq) output = r;
    } catch (e) {
      if (id === seq) {
        error = errMsg(e);
        output = "";
      }
    }
  }

  $effect(() => {
    void [input, target];
    run();
  });
</script>

<ToolPanel
  title="简繁转换"
  description="基于 MediaWiki + OpenCC 规则的词组级简繁/地区词转换；选择目标变体即可（自动判断来源）。"
>
  <div class="mb-3 flex flex-wrap items-center gap-3">
    <span class="{cls.label} w-14 shrink-0">转换为</span>
    <SegmentedControl
      bind:value={target}
      options={[
        { label: "简体", value: "hans" },
        { label: "繁体", value: "hant" },
        { label: "台湾正体", value: "tw" },
        { label: "香港繁体", value: "hk" },
        { label: "大陆简体", value: "cn" },
      ]}
    />
  </div>

  {#if error}
    <p class="mb-3 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-600 dark:bg-red-950/40 dark:text-red-400">
      {error}
    </p>
  {/if}

  <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
    <div>
      <div class="mb-1 flex h-7 items-center"><span class={cls.label}>输入</span></div>
      <textarea
        bind:value={input}
        spellcheck="false"
        class="{cls.field} h-72 resize-none leading-relaxed"
      ></textarea>
    </div>
    <div>
      <div class="mb-1 flex h-7 items-center justify-between">
        <span class={cls.label}>输出</span>
        {#if output}<CopyButton text={output} />{/if}
      </div>
      <textarea
        value={output}
        readonly
        spellcheck="false"
        class="{cls.field} h-72 resize-none bg-slate-50 leading-relaxed dark:bg-slate-900/60"
      ></textarea>
    </div>
  </div>
</ToolPanel>

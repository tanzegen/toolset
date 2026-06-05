<script lang="ts">
  import { api, errMsg } from "../ipc";
  import ToolPanel from "../components/ToolPanel.svelte";
  import SegmentedControl from "../components/SegmentedControl.svelte";
  import CopyButton from "../components/CopyButton.svelte";
  import { cls } from "../ui";
  import { persist } from "../persist.svelte";

  let tool = $state("base64");
  let direction = $state("encode");
  let variant = $state("standard"); // base64: standard / urlsafe
  let input = $state("Hello, 世界");
  let output = $state("");
  let error = $state("");
  let seq = 0;

  persist("encoding", {
    save: () => ({ tool, direction, variant, input }),
    load: (s) => {
      tool = s.tool ?? tool;
      direction = s.direction ?? direction;
      variant = s.variant ?? variant;
      input = s.input ?? input;
    },
  });

  async function run() {
    const id = ++seq;
    error = "";
    if (!input) {
      output = "";
      return;
    }
    try {
      let r: string;
      const urlSafe = variant === "urlsafe";
      if (tool === "base64") {
        r =
          direction === "encode"
            ? await api.base64Encode(input, urlSafe)
            : await api.base64Decode(input, urlSafe);
      } else {
        r =
          direction === "encode"
            ? await api.urlEncode(input)
            : await api.urlDecode(input);
      }
      if (id === seq) output = r;
    } catch (e) {
      if (id === seq) {
        error = errMsg(e);
        output = "";
      }
    }
  }

  $effect(() => {
    void [tool, direction, variant, input];
    run();
  });
</script>

<ToolPanel
  title="Base64 / URL 编解码"
  description="Base64（标准 / URL-safe）与 URL 百分号编解码，全 UTF-8。"
>
  <div class="mb-3 flex flex-wrap items-center gap-3">
    <SegmentedControl
      bind:value={tool}
      options={[
        { label: "Base64", value: "base64" },
        { label: "URL", value: "url" },
      ]}
    />
    <SegmentedControl
      bind:value={direction}
      options={[
        { label: "编码", value: "encode" },
        { label: "解码", value: "decode" },
      ]}
    />
    {#if tool === "base64"}
      <SegmentedControl
        bind:value={variant}
        options={[
          { label: "标准", value: "standard" },
          { label: "URL-safe", value: "urlsafe" },
        ]}
      />
    {/if}
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
        class="{cls.field} h-64 resize-none font-mono leading-relaxed"
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
        class="{cls.field} h-64 resize-none bg-slate-50 font-mono leading-relaxed dark:bg-slate-900/60"
      ></textarea>
    </div>
  </div>
</ToolPanel>

<script lang="ts">
  import { api, errMsg } from "../ipc";
  import ToolPanel from "../components/ToolPanel.svelte";
  import SegmentedControl from "../components/SegmentedControl.svelte";
  import CopyButton from "../components/CopyButton.svelte";
  import JsonTree from "../components/JsonTree.svelte";
  import JsonFieldTree from "../components/JsonFieldTree.svelte";
  import { buildFieldTree, leafPaths, type FieldNode } from "../jsonfields";
  import { cls } from "../ui";
  import { persist } from "../persist.svelte";

  let action = $state("format");
  let indent = $state(2);
  let input = $state(
    '[{"id":1,"name":"Alice","age":30,"addr":{"city":"BJ","zip":"100"},"tags":["a","b"]},{"id":2,"name":"Bob","age":25,"addr":{"city":"SH","zip":"200"},"tags":["c"]}]',
  );
  let output = $state("");
  let error = $state("");
  let treeDepth = $state(2);
  let picked = $state<string[]>([]); // 字段提取：勾选的叶子路径
  let seq = 0;

  persist("json", {
    save: () => ({ action, indent, input, treeDepth, picked }),
    load: (s) => {
      action = s.action ?? action;
      indent = s.indent ?? indent;
      input = s.input ?? input;
      treeDepth = s.treeDepth ?? treeDepth;
      picked = s.picked ?? picked;
    },
  });

  // 树视图：前端解析（与格式化/压缩走 Rust 互不影响）
  const parsed = $derived.by(() => {
    if (action !== "tree") return null;
    if (!input.trim()) return { ok: true as const, data: undefined as unknown };
    try {
      return { ok: true as const, data: JSON.parse(input) as unknown };
    } catch (e) {
      return { ok: false as const, msg: e instanceof Error ? e.message : String(e) };
    }
  });

  // 字段提取：前端解析推断字段树（驱动勾选 UI）；真正投影由后端 json_pick 完成。
  const fieldTree = $derived.by(() => {
    if (action !== "pick") return null;
    if (!input.trim()) return { ok: true as const, nodes: [] as FieldNode[] };
    try {
      return { ok: true as const, nodes: buildFieldTree(JSON.parse(input) as unknown) };
    } catch (e) {
      return { ok: false as const, msg: e instanceof Error ? e.message : String(e) };
    }
  });
  const allLeaves = $derived(fieldTree?.ok ? fieldTree.nodes.flatMap(leafPaths) : []);

  function toggleNode(n: FieldNode) {
    const leaves = leafPaths(n);
    if (leaves.every((p) => picked.includes(p))) picked = picked.filter((p) => !leaves.includes(p));
    else picked = [...new Set([...picked, ...leaves])];
  }

  const showError = $derived(
    action === "tree"
      ? parsed && !parsed.ok
        ? parsed.msg
        : ""
      : action === "pick"
        ? fieldTree && !fieldTree.ok
          ? fieldTree.msg
          : ""
        : error,
  );
  const showOk = $derived(
    action === "tree"
      ? !!(parsed && parsed.ok && parsed.data !== undefined)
      : action === "pick"
        ? !!(fieldTree && fieldTree.ok && fieldTree.nodes.length)
        : !!output,
  );

  async function run() {
    if (action === "tree") {
      output = "";
      error = "";
      return;
    }
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
          : action === "minify"
            ? await api.jsonMinify(input)
            : await api.jsonPick(input, picked);
      if (id === seq) output = r;
    } catch (e) {
      if (id === seq) {
        error = errMsg(e);
        output = "";
      }
    }
  }

  $effect(() => {
    void [action, indent, input, picked];
    run();
  });
</script>

<ToolPanel title="JSON 工具" description="格式化、压缩、树视图、字段提取（勾选字段拼成新 JSON），错误带行列定位。">
  <div class="mb-3 flex flex-wrap items-center gap-3">
    <SegmentedControl
      bind:value={action}
      options={[
        { label: "格式化", value: "format" },
        { label: "压缩", value: "minify" },
        { label: "树视图", value: "tree" },
        { label: "字段提取", value: "pick" },
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
    {#if action === "tree"}
      <button class={cls.btn} onclick={() => (treeDepth = 99)}>全部展开</button>
      <button class={cls.btn} onclick={() => (treeDepth = 1)}>折叠</button>
    {/if}
    {#if action === "pick"}
      <button class={cls.btn} onclick={() => (picked = allLeaves)}>全选</button>
      <button class={cls.btn} onclick={() => (picked = [])}>清空</button>
      <span class="text-xs text-slate-400">已选 {picked.length}</span>
    {/if}
    {#if showError}
      <span class="rounded-md bg-red-50 px-2 py-1 text-xs text-red-600 dark:bg-red-950/40 dark:text-red-400">
        {showError}
      </span>
    {:else if showOk}
      <span class="rounded-md bg-emerald-50 px-2 py-1 text-xs text-emerald-600 dark:bg-emerald-950/40 dark:text-emerald-400">
        ✓ 合法 JSON
      </span>
    {/if}
  </div>

  {#if action === "pick"}
    <div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
      <div>
        <div class="mb-1 flex h-7 items-center"><span class={cls.label}>输入</span></div>
        <textarea
          bind:value={input}
          spellcheck="false"
          class="{cls.field} h-80 resize-none font-mono leading-relaxed"
        ></textarea>
      </div>
      <div>
        <div class="mb-1 flex h-7 items-center"><span class={cls.label}>字段（勾选要保留的）</span></div>
        <div class="{cls.card} h-80 overflow-auto px-1 py-2">
          {#if fieldTree?.ok && fieldTree.nodes.length}
            <JsonFieldTree nodes={fieldTree.nodes} {picked} ontoggle={toggleNode} />
          {:else if !input.trim()}
            <span class="px-2 text-slate-400">输入对象或对象数组后在此勾选字段</span>
          {:else if fieldTree?.ok}
            <span class="px-2 text-slate-400">未发现可选字段（顶层不是对象/对象数组）</span>
          {/if}
        </div>
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
          class="{cls.field} h-80 resize-none bg-slate-50 font-mono leading-relaxed dark:bg-slate-900/60"
        ></textarea>
      </div>
    </div>
  {:else}
    <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
      <div>
        <div class="mb-1 flex h-7 items-center"><span class={cls.label}>输入</span></div>
        <textarea
          bind:value={input}
          spellcheck="false"
          class="{cls.field} h-80 resize-none font-mono leading-relaxed"
        ></textarea>
      </div>
      <div>
        <div class="mb-1 flex h-7 items-center justify-between">
          <span class={cls.label}>{action === "tree" ? "树视图" : "输出"}</span>
          {#if action !== "tree" && output}<CopyButton text={output} />{/if}
        </div>
        {#if action === "tree"}
          <div
            class="{cls.card} h-80 overflow-auto px-3 py-2 font-mono text-sm leading-relaxed"
          >
            {#if parsed && parsed.ok && parsed.data !== undefined}
              {#key treeDepth}
                <JsonTree data={parsed.data} defaultDepth={treeDepth} />
              {/key}
            {:else if !input.trim()}
              <span class="text-slate-400">输入 JSON 后在此查看树结构</span>
            {/if}
          </div>
        {:else}
          <textarea
            value={output}
            readonly
            spellcheck="false"
            class="{cls.field} h-80 resize-none bg-slate-50 font-mono leading-relaxed dark:bg-slate-900/60"
          ></textarea>
        {/if}
      </div>
    </div>
  {/if}
</ToolPanel>

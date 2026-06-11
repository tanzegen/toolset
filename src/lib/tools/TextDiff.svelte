<script lang="ts">
  import { api, errMsg, type DiffResult } from "../ipc";
  import ToolPanel from "../components/ToolPanel.svelte";
  import { cls } from "../ui";
  import { persist } from "../persist.svelte";

  let left = $state("line one\nline two\nline three");
  let right = $state("line one\nline 2\nline three\nline four");
  let result = $state<DiffResult | null>(null);
  let error = $state("");
  let seq = 0;

  persist("diff", {
    save: () => ({ left, right }),
    load: (s) => {
      left = s.left ?? left;
      right = s.right ?? right;
    },
  });

  async function run() {
    const id = ++seq;
    error = "";
    try {
      const r = await api.textDiff(left, right);
      if (id === seq) result = r;
    } catch (e) {
      if (id === seq) error = errMsg(e);
    }
  }

  $effect(() => {
    void [left, right];
    run();
  });

  const sign: Record<string, string> = { equal: " ", insert: "+", delete: "-" };
  function rowClass(tag: string): string {
    if (tag === "insert") return "bg-emerald-50 dark:bg-emerald-950/30";
    if (tag === "delete") return "bg-red-50 dark:bg-red-950/30";
    return "";
  }
  function signClass(tag: string): string {
    if (tag === "insert") return "text-emerald-600 dark:text-emerald-400";
    if (tag === "delete") return "text-red-600 dark:text-red-400";
    return "text-slate-300 dark:text-slate-600";
  }
</script>

<ToolPanel wide title="文本 Diff" description="按行比较两段文本，高亮新增与删除。">
  <div class="grid grid-cols-1 gap-4 lg:grid-cols-2">
    <div>
      <div class="{cls.label} mb-1">原文</div>
      <textarea
        bind:value={left}
        spellcheck="false"
        class="{cls.field} {cls.editorMid} resize-none font-mono leading-relaxed"
      ></textarea>
    </div>
    <div>
      <div class="{cls.label} mb-1">对比</div>
      <textarea
        bind:value={right}
        spellcheck="false"
        class="{cls.field} {cls.editorMid} resize-none font-mono leading-relaxed"
      ></textarea>
    </div>
  </div>

  {#if error}
    <p class="mt-4 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-600 dark:bg-red-950/40 dark:text-red-400">
      {error}
    </p>
  {/if}

  {#if result}
    <div class="mt-4 flex items-center gap-2 text-xs">
      <span class="rounded bg-emerald-50 px-2 py-1 font-medium text-emerald-600 dark:bg-emerald-950/40 dark:text-emerald-400">+{result.added}</span>
      <span class="rounded bg-red-50 px-2 py-1 font-medium text-red-600 dark:bg-red-950/40 dark:text-red-400">-{result.removed}</span>
    </div>
    <div class="{cls.card} mt-2 max-h-[calc(32vh_+_16rem)] overflow-auto py-1">
      {#each result.rows as row, i (i)}
        <div class="flex font-mono text-xs leading-relaxed {rowClass(row.tag)}">
          <span class="w-10 shrink-0 select-none px-1 text-right text-slate-400">{row.left_no ?? ""}</span>
          <span class="w-10 shrink-0 select-none px-1 text-right text-slate-400">{row.right_no ?? ""}</span>
          <span class="w-4 shrink-0 select-none text-center {signClass(row.tag)}">{sign[row.tag]}</span>
          <span class="flex-1 whitespace-pre-wrap break-all pr-2 text-slate-700 dark:text-slate-200">{row.text || " "}</span>
        </div>
      {/each}
    </div>
  {/if}
</ToolPanel>

<script lang="ts">
  import { api, errMsg, type RegexResult } from "../ipc";
  import ToolPanel from "../components/ToolPanel.svelte";
  import SegmentedControl from "../components/SegmentedControl.svelte";
  import { cls } from "../ui";
  import { persist } from "../persist.svelte";

  let pattern = $state("(\\w+)@(\\w+)\\.(\\w+)");
  let text = $state("联系 alice@example.com 或 bob@test.org");
  let mode = $state("match");
  let replacement = $state("$1 [at] $2.$3");
  let fi = $state(false);
  let fm = $state(false);
  let fs = $state(false);
  let fx = $state(false);
  let result = $state<RegexResult | null>(null);
  let error = $state("");
  let seq = 0;

  persist("regex", {
    save: () => ({ pattern, text, mode, replacement, fi, fm, fs, fx }),
    load: (s) => {
      pattern = s.pattern ?? pattern;
      text = s.text ?? text;
      mode = s.mode ?? mode;
      replacement = s.replacement ?? replacement;
      fi = s.fi ?? fi;
      fm = s.fm ?? fm;
      fs = s.fs ?? fs;
      fx = s.fx ?? fx;
    },
  });

  const flags = $derived(
    (fi ? "i" : "") + (fm ? "m" : "") + (fs ? "s" : "") + (fx ? "x" : ""),
  );

  async function run() {
    const id = ++seq;
    error = "";
    if (!pattern) {
      result = null;
      return;
    }
    try {
      const repl = mode === "replace" ? replacement : null;
      const r = await api.regexTest(pattern, flags, text, repl);
      if (id === seq) result = r;
    } catch (e) {
      if (id === seq) {
        error = errMsg(e);
        result = null;
      }
    }
  }

  $effect(() => {
    void [pattern, text, flags, mode, replacement];
    run();
  });
</script>

<ToolPanel
  wide
  title="正则测试器"
  description="实时匹配、捕获组与替换。RE2 语义（不支持反向引用 / 环视）。"
>
  <div class="space-y-3">
    <input bind:value={pattern} placeholder="正则表达式" class="{cls.field} font-mono" />
    <div class="flex flex-wrap items-center gap-4">
      <SegmentedControl
        bind:value={mode}
        options={[
          { label: "匹配", value: "match" },
          { label: "替换", value: "replace" },
        ]}
      />
      <label class="flex items-center gap-1.5 text-sm text-slate-600 dark:text-slate-300"><input type="checkbox" bind:checked={fi} /> i 忽略大小写</label>
      <label class="flex items-center gap-1.5 text-sm text-slate-600 dark:text-slate-300"><input type="checkbox" bind:checked={fm} /> m 多行</label>
      <label class="flex items-center gap-1.5 text-sm text-slate-600 dark:text-slate-300"><input type="checkbox" bind:checked={fs} /> s 点匹配换行</label>
      <label class="flex items-center gap-1.5 text-sm text-slate-600 dark:text-slate-300"><input type="checkbox" bind:checked={fx} /> x 忽略空白</label>
    </div>
    {#if mode === "replace"}
      <input bind:value={replacement} placeholder="替换为（$1 $2 引用分组）" class="{cls.field} font-mono" />
    {/if}
    <textarea
      bind:value={text}
      spellcheck="false"
      class="{cls.field} {cls.editorMid} resize-none font-mono leading-relaxed"
    ></textarea>
  </div>

  {#if error}
    <p class="mt-4 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-600 dark:bg-red-950/40 dark:text-red-400">
      {error}
    </p>
  {/if}

  {#if result}
    <div class="mt-4 flex items-center gap-2">
      <span class="rounded-md bg-indigo-50 px-2 py-1 text-xs font-medium text-indigo-600 dark:bg-indigo-950/40 dark:text-indigo-300">
        {result.count} 处匹配
      </span>
    </div>

    {#if mode === "replace"}
      <div class="{cls.card} mt-3 px-4 py-3">
        <div class="mb-1 text-xs text-slate-400">替换结果</div>
        <div class="select-text whitespace-pre-wrap break-all font-mono text-sm text-slate-700 dark:text-slate-200">{result.replaced}</div>
      </div>
    {:else}
      <div class="{cls.card} mt-3 max-h-[calc(30vh_+_14rem)] overflow-y-auto px-4 py-2">
        {#each result.matches as m (m.index)}
          <div class="border-b border-slate-100 py-2 last:border-0 dark:border-slate-800">
            <div class="flex items-center gap-2">
              <span class="text-xs text-slate-400">#{m.index + 1} · [{m.start},{m.end}]</span>
              <span class="font-mono text-sm font-medium text-indigo-600 dark:text-indigo-300">{m.text}</span>
            </div>
            {#if m.groups.length}
              <div class="mt-1 flex flex-wrap gap-2 pl-2">
                {#each m.groups as g (g.name)}
                  <span class="rounded bg-slate-100 px-2 py-0.5 text-xs dark:bg-slate-800">
                    <span class="text-slate-400">{g.name}</span>
                    <span class="ml-1 font-mono text-slate-700 dark:text-slate-200">{g.value ?? "∅"}</span>
                  </span>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
        {#if result.matches.length === 0}
          <p class="py-2 text-sm text-slate-400">无匹配</p>
        {/if}
      </div>
    {/if}
  {/if}
</ToolPanel>

<script lang="ts">
  import { api, errMsg, type CaseResult } from "../ipc";
  import ToolPanel from "../components/ToolPanel.svelte";
  import ResultRow from "../components/ResultRow.svelte";
  import { cls } from "../ui";

  let input = $state("user_profile_id");
  let result = $state<CaseResult | null>(null);
  let error = $state("");
  let seq = 0;

  async function run() {
    const id = ++seq;
    error = "";
    if (!input.trim()) {
      result = null;
      return;
    }
    try {
      const r = await api.convertCase(input);
      if (id === seq) result = r;
    } catch (e) {
      if (id === seq) {
        error = errMsg(e);
        result = null;
      }
    }
  }

  $effect(() => {
    void input;
    run();
  });
</script>

<ToolPanel
  title="命名风格转换"
  description="一段标识符 → 各种命名风格（识别空格 / 下划线 / 连字符 / 驼峰边界）。"
>
  <input
    bind:value={input}
    placeholder="如 user_profile_id、getHTTPResponse、order-item"
    class="{cls.field} font-mono"
  />

  {#if error}
    <p class="mt-4 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-600 dark:bg-red-950/40 dark:text-red-400">
      {error}
    </p>
  {/if}

  {#if result}
    <div class="{cls.card} mt-5 px-4 py-2">
      <ResultRow label="camelCase" value={result.lower_camel} />
      <ResultRow label="PascalCase" value={result.upper_camel} />
      <ResultRow label="snake_case" value={result.snake} />
      <ResultRow label="SCREAMING_SNAKE" value={result.shouty_snake} />
      <ResultRow label="kebab-case" value={result.kebab} />
      <ResultRow label="SCREAMING-KEBAB" value={result.shouty_kebab} />
      <ResultRow label="Train-Case" value={result.train} />
      <ResultRow label="Title Case" value={result.title} />
    </div>
  {/if}
</ToolPanel>

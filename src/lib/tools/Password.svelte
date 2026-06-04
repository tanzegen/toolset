<script lang="ts">
  import { onMount } from "svelte";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { api, errMsg, type PasswordResult } from "../ipc";
  import ToolPanel from "../components/ToolPanel.svelte";
  import CopyButton from "../components/CopyButton.svelte";
  import { cls } from "../ui";

  let digits = $state(true);
  let lower = $state(true);
  let upper = $state(true);
  let symbols = $state(false);
  let length = $state(16);
  let count = $state(5);
  let mustInclude = $state("");
  let exclude = $state("");
  let excludeConfusable = $state(true);

  let result = $state<PasswordResult | null>(null);
  let error = $state("");

  async function gen() {
    error = "";
    try {
      result = await api.generatePassword({
        digits,
        lower,
        upper,
        symbols,
        length,
        count,
        mustInclude,
        exclude,
        excludeConfusable,
      });
    } catch (e) {
      error = errMsg(e);
      result = null;
    }
  }

  onMount(gen);

  const checkboxClass =
    "flex items-center gap-1.5 text-sm text-slate-600 dark:text-slate-300";
</script>

<ToolPanel title="随机密码" description="自定义字符集、长度、数量、必含/排除字符；默认排除易混淆字符（0Oo1lLiI）。">
  <div class="space-y-3">
    <div class="flex flex-wrap items-center gap-4">
      <span class="{cls.label} w-14 shrink-0">字符集</span>
      <label class={checkboxClass}><input type="checkbox" bind:checked={digits} /> 数字</label>
      <label class={checkboxClass}><input type="checkbox" bind:checked={lower} /> 小写字母</label>
      <label class={checkboxClass}><input type="checkbox" bind:checked={upper} /> 大写字母</label>
      <label class={checkboxClass}><input type="checkbox" bind:checked={symbols} /> 符号</label>
    </div>

    <div class="flex flex-wrap items-center gap-4">
      <span class="{cls.label} w-14 shrink-0">长度</span>
      <input type="number" min="1" max="256" bind:value={length} class="{cls.field} w-24 font-mono" />
      <span class={cls.label}>数量</span>
      <input type="number" min="1" max="100" bind:value={count} class="{cls.field} w-24 font-mono" />
    </div>

    <div class="flex flex-wrap items-center gap-3">
      <span class="{cls.label} w-14 shrink-0">必含</span>
      <input bind:value={mustInclude} placeholder="每个字符至少出现一次，如 @#" class="{cls.field} flex-1 font-mono" />
    </div>
    <div class="flex flex-wrap items-center gap-3">
      <span class="{cls.label} w-14 shrink-0">排除</span>
      <input bind:value={exclude} placeholder="额外排除这些字符" class="{cls.field} flex-1 font-mono" />
    </div>

    <div class="flex flex-wrap items-center gap-4">
      <label class={checkboxClass}>
        <input type="checkbox" bind:checked={excludeConfusable} /> 排除易混淆字符（0Oo1lLiI）
      </label>
      <button class={cls.btnPrimary} onclick={gen}>生成</button>
    </div>
  </div>

  {#if error}
    <p class="mt-4 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-600 dark:bg-red-950/40 dark:text-red-400">
      {error}
    </p>
  {/if}

  {#if result && result.passwords.length}
    <div class="mt-4 flex items-center justify-between">
      <span class="text-xs text-slate-400">字符池大小 {result.pool_size}</span>
      <button class={cls.btn} onclick={() => writeText(result!.passwords.join("\n"))}>复制全部</button>
    </div>
    <div class="{cls.card} mt-2 px-4 py-2">
      {#each result.passwords as p, i (i)}
        <div class="flex items-center gap-3 border-b border-slate-100 py-1.5 last:border-0 dark:border-slate-800">
          <span class="flex-1 select-text break-all font-mono text-sm text-slate-700 dark:text-slate-200">{p}</span>
          <CopyButton text={p} />
        </div>
      {/each}
    </div>
  {/if}
</ToolPanel>

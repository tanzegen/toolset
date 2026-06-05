<script lang="ts">
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import { api, errMsg, type HashResult } from "../ipc";
  import ToolPanel from "../components/ToolPanel.svelte";
  import SegmentedControl from "../components/SegmentedControl.svelte";
  import ResultRow from "../components/ResultRow.svelte";
  import CopyButton from "../components/CopyButton.svelte";
  import { cls } from "../ui";
  import { persist } from "../persist.svelte";

  let tool = $state("hash");

  // 哈希
  let input = $state("abc");
  let hash = $state<HashResult | null>(null);
  let seq = 0;

  $effect(() => {
    void input;
    void tool;
    if (tool !== "hash") return;
    const id = ++seq;
    if (!input) {
      hash = null;
      return;
    }
    api.hashText(input).then((r) => {
      if (id === seq) hash = r;
    });
  });

  // UUID
  let count = $state(5);
  let uuids = $state<string[]>([]);
  let error = $state("");

  // 持久化当前子工具、待哈希文本与 UUID 数量；UUID 结果属生成内容，不落盘。
  persist("hashing", {
    save: () => ({ tool, input, count }),
    load: (s) => {
      tool = s.tool ?? tool;
      input = s.input ?? input;
      count = s.count ?? count;
    },
  });

  async function genUuid() {
    error = "";
    try {
      uuids = await api.uuidV4(count);
    } catch (e) {
      error = errMsg(e);
    }
  }
</script>

<ToolPanel
  title="哈希 / UUID"
  description="文本 MD5 / SHA1 / SHA256 / SHA512，以及 UUID v4 生成。"
>
  <div class="mb-4">
    <SegmentedControl
      bind:value={tool}
      options={[
        { label: "文本哈希", value: "hash" },
        { label: "UUID 生成", value: "uuid" },
      ]}
    />
  </div>

  {#if tool === "hash"}
    <div class="{cls.label} mb-1">输入文本</div>
    <textarea
      bind:value={input}
      spellcheck="false"
      class="{cls.field} h-32 resize-none font-mono leading-relaxed"
    ></textarea>
    {#if hash}
      <div class="{cls.card} mt-4 px-4 py-2">
        <ResultRow label="MD5" value={hash.md5} />
        <ResultRow label="SHA-1" value={hash.sha1} />
        <ResultRow label="SHA-256" value={hash.sha256} />
        <ResultRow label="SHA-512" value={hash.sha512} />
      </div>
    {/if}
  {:else}
    <div class="mb-4 flex flex-wrap items-end gap-3">
      <div>
        <div class="{cls.label} mb-1">数量 (1–100)</div>
        <input
          type="number"
          min="1"
          max="100"
          bind:value={count}
          class="{cls.field} w-28 font-mono"
        />
      </div>
      <button class={cls.btnPrimary} onclick={genUuid}>生成</button>
      {#if uuids.length}
        <button class={cls.btn} onclick={() => writeText(uuids.join("\n"))}>
          复制全部
        </button>
      {/if}
    </div>
    {#if error}
      <p class="mb-3 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-600 dark:bg-red-950/40 dark:text-red-400">
        {error}
      </p>
    {/if}
    {#if uuids.length}
      <div class="{cls.card} px-4 py-2">
        {#each uuids as u (u)}
          <div class="flex items-center gap-3 border-b border-slate-100 py-1.5 last:border-0 dark:border-slate-800">
            <span class="flex-1 select-text break-all font-mono text-sm text-slate-700 dark:text-slate-200">{u}</span>
            <CopyButton text={u} />
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</ToolPanel>

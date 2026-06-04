<script lang="ts">
  import { untrack } from "svelte";
  import Node from "./JsonTree.svelte"; // 递归自引用

  let {
    data,
    name = undefined,
    depth = 0,
    defaultDepth = 2,
  }: {
    data: unknown;
    name?: string | number;
    depth?: number;
    defaultDepth?: number;
  } = $props();

  const isArray = $derived(Array.isArray(data));
  const isContainer = $derived(
    Array.isArray(data) || (data !== null && typeof data === "object"),
  );

  const entries = $derived(
    Array.isArray(data)
      ? (data as unknown[]).map((v, i) => [i, v] as [string | number, unknown])
      : data !== null && typeof data === "object"
        ? (Object.entries(data as Record<string, unknown>) as [
            string | number,
            unknown,
          ][])
        : ([] as [string | number, unknown][]),
  );

  // 仅取初始值：节点展开状态由用户切换；defaultDepth 变化通过 {#key} 重挂载生效。
  let open = $state(untrack(() => depth < defaultDepth));

  const keyLabel = $derived(
    name === undefined
      ? ""
      : typeof name === "number"
        ? String(name)
        : `"${name}"`,
  );

  function valueClass(v: unknown): string {
    if (typeof v === "string") return "text-emerald-600 dark:text-emerald-400";
    if (typeof v === "number") return "text-blue-600 dark:text-blue-400";
    if (typeof v === "boolean") return "text-purple-600 dark:text-purple-400";
    if (v === null) return "italic text-slate-400";
    return "text-slate-700 dark:text-slate-200";
  }
  function valueText(v: unknown): string {
    if (typeof v === "string") return `"${v}"`;
    if (v === null) return "null";
    return String(v);
  }
</script>

{#if isContainer}
  <div>
    <button
      type="button"
      onclick={() => (open = !open)}
      class="inline-flex items-center gap-1 rounded px-0.5 text-left hover:bg-slate-100 dark:hover:bg-slate-700/50"
    >
      <span class="inline-block w-3 select-none text-slate-400">{open ? "▾" : "▸"}</span>
      {#if name !== undefined}
        <span class="text-slate-500 dark:text-slate-400">{keyLabel}:</span>
      {/if}
      <span class="text-slate-400">
        {isArray ? "[" : "{"}{#if !open}<span class="px-0.5">…</span>{isArray ? "]" : "}"}{/if}
      </span>
      <span class="rounded bg-slate-100 px-1 text-[10px] text-slate-500 dark:bg-slate-700 dark:text-slate-400">
        {entries.length}
        {isArray ? "项" : "键"}
      </span>
    </button>
    {#if open}
      <div class="ml-1.5 border-l border-slate-200 pl-3 dark:border-slate-700">
        {#each entries as [k, v] (k)}
          <Node data={v} name={k} depth={depth + 1} {defaultDepth} />
        {/each}
      </div>
      <div class="pl-1 text-slate-400">{isArray ? "]" : "}"}</div>
    {/if}
  </div>
{:else}
  <div>
    {#if name !== undefined}
      <span class="text-slate-500 dark:text-slate-400">{keyLabel}:</span>
    {/if}
    <span class={valueClass(data)}>{valueText(data)}</span>
  </div>
{/if}

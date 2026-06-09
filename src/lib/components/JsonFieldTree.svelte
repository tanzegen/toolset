<script lang="ts">
  import { type FieldNode, leafPaths } from "../jsonfields";
  import Self from "./JsonFieldTree.svelte"; // 自引用以递归渲染子树

  let {
    nodes,
    picked,
    ontoggle,
    depth = 0,
  }: {
    nodes: FieldNode[];
    picked: string[];
    ontoggle: (n: FieldNode) => void;
    depth?: number;
  } = $props();

  // 展开状态：默认仅第一层展开（depth<1），可逐节点切换。
  let open = $state<Record<string, boolean>>({});
  const isOpen = (p: string) => open[p] ?? depth < 1;

  function tri(n: FieldNode): "on" | "off" | "mix" {
    const leaves = leafPaths(n);
    const sel = leaves.filter((p) => picked.includes(p)).length;
    return sel === 0 ? "off" : sel === leaves.length ? "on" : "mix";
  }

  // indeterminate 是 DOM 属性而非特性，用 action 同步。
  function indet(node: HTMLInputElement, v: boolean) {
    node.indeterminate = v;
    return { update: (v2: boolean) => (node.indeterminate = v2) };
  }
</script>

{#each nodes as n (n.path)}
  {@const st = tri(n)}
  <div style="padding-left:{depth * 14}px">
    <div class="flex items-center gap-1 rounded px-1 py-0.5 hover:bg-slate-50 dark:hover:bg-slate-800/50">
      {#if n.children.length}
        <button class="w-4 shrink-0 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200" onclick={() => (open[n.path] = !isOpen(n.path))} aria-label="展开/折叠">
          {isOpen(n.path) ? "▾" : "▸"}
        </button>
      {:else}
        <span class="w-4 shrink-0"></span>
      {/if}
      <label class="flex min-w-0 flex-1 cursor-pointer items-center gap-1.5">
        <input
          type="checkbox"
          checked={st === "on"}
          use:indet={st === "mix"}
          onchange={() => ontoggle(n)}
          class="h-3.5 w-3.5 shrink-0 accent-indigo-500"
        />
        <span class="truncate font-mono text-xs text-slate-700 dark:text-slate-200">{n.key}</span>
        <span class="shrink-0 text-[10px] text-slate-400">{n.type}</span>
        {#if n.sample}<span class="truncate text-[10px] text-slate-400">e.g. {n.sample}</span>{/if}
      </label>
    </div>
    {#if n.children.length && isOpen(n.path)}
      <Self nodes={n.children} {picked} {ontoggle} depth={depth + 1} />
    {/if}
  </div>
{/each}

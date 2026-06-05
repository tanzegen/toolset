<script lang="ts">
  import Icon from "../Icon.svelte";

  let {
    x,
    y,
    items,
    onclose,
  }: {
    x: number;
    y: number;
    items: { label: string; icon?: string; danger?: boolean; onclick: () => void }[];
    onclose: () => void;
  } = $props();
</script>

<!-- 点击/右键空白处、Esc、窗口缩放都关闭菜单 -->
<svelte:window
  onclick={onclose}
  oncontextmenu={onclose}
  onresize={onclose}
  onkeydown={(e) => e.key === "Escape" && onclose()}
/>

<div
  class="fixed z-50 min-w-36 overflow-hidden rounded-lg border border-slate-200 bg-white py-1 text-sm shadow-xl dark:border-slate-700 dark:bg-slate-800"
  style="left: {x}px; top: {y}px"
  role="menu"
  tabindex="-1"
>
  {#each items as it (it.label)}
    <button
      class="flex w-full items-center gap-2 px-3 py-1.5 text-left transition hover:bg-slate-100 dark:hover:bg-slate-700 {it.danger
        ? 'text-red-600 dark:text-red-400'
        : 'text-slate-700 dark:text-slate-200'}"
      role="menuitem"
      onclick={(e) => {
        e.stopPropagation();
        onclose();
        it.onclick();
      }}
    >
      {#if it.icon}<Icon name={it.icon} size={14} />{/if}
      <span>{it.label}</span>
    </button>
  {/each}
</div>

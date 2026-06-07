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

  // 夹取到视口内：靠右/靠下打开时整体左移/上移，避免菜单溢出窗口被裁切点不到。
  let el = $state<HTMLDivElement>();
  let px = $state(x);
  let py = $state(y);
  $effect(() => {
    void x;
    void y;
    if (!el) return;
    const m = 8;
    px = Math.max(m, Math.min(x, window.innerWidth - el.offsetWidth - m));
    py = Math.max(m, Math.min(y, window.innerHeight - el.offsetHeight - m));
  });
</script>

<!-- 点击/右键空白处、Esc、窗口缩放都关闭菜单 -->
<svelte:window
  onclick={onclose}
  oncontextmenu={onclose}
  onresize={onclose}
  onkeydown={(e) => e.key === "Escape" && onclose()}
/>

<div
  bind:this={el}
  class="fixed z-50 min-w-36 overflow-hidden rounded-lg border border-slate-200 bg-white py-1 text-sm shadow-xl dark:border-slate-700 dark:bg-slate-800"
  style="left: {px}px; top: {py}px"
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

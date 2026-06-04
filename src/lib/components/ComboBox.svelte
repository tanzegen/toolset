<script lang="ts">
  import { cls } from "../ui";

  // 自绘下拉组合框：聚焦清空 → 展示全部预设可筛选；可键盘导航；可自由输入。
  // strict=true（如时区）：移开时若不在选项内则还原上次值；strict=false（如端点）：任意非空均接受。
  let {
    value = $bindable(),
    options = [],
    placeholder = "",
    strict = false,
  }: {
    value: string;
    options?: string[];
    placeholder?: string;
    strict?: boolean;
  } = $props();

  const MAX = 100;

  let draft = $state(value);
  let open = $state(false);
  let highlighted = $state(-1);
  let focused = false;

  const filtered = $derived(
    draft.trim() === ""
      ? options
      : options.filter((o) => o.toLowerCase().includes(draft.toLowerCase())),
  );
  const shown = $derived(filtered.slice(0, MAX));

  // 外部 value 变化且未聚焦时同步显示
  $effect(() => {
    if (!focused) draft = value;
  });

  function commitFromDraft(): boolean {
    const v = draft.trim();
    if (!v) return false;
    if (strict) {
      if (options.length === 0 || options.includes(v)) {
        value = v;
        return true;
      }
      return false;
    }
    value = v;
    return true;
  }

  function selectOption(o: string) {
    value = o;
    draft = o;
    open = false;
    highlighted = -1;
  }
</script>

<div class="relative w-full min-w-0 flex-1">
  <input
    value={draft}
    autocomplete="off"
    spellcheck="false"
    {placeholder}
    class="{cls.field} font-mono"
    oninput={(e) => {
      draft = (e.currentTarget as HTMLInputElement).value;
      open = true;
      highlighted = -1;
      commitFromDraft();
    }}
    onfocus={() => {
      focused = true;
      draft = "";
      open = true;
      highlighted = -1;
    }}
    onclick={() => (open = true)}
    onblur={() => {
      focused = false;
      open = false;
      if (!commitFromDraft()) draft = value;
    }}
    onkeydown={(e) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        open = true;
        highlighted = Math.min(highlighted + 1, shown.length - 1);
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        highlighted = Math.max(highlighted - 1, 0);
      } else if (e.key === "Enter") {
        if (open && highlighted >= 0 && highlighted < shown.length) {
          e.preventDefault();
          selectOption(shown[highlighted]);
          (e.currentTarget as HTMLInputElement).blur();
        } else if (commitFromDraft()) {
          open = false;
        }
      } else if (e.key === "Escape") {
        open = false;
        draft = value;
        (e.currentTarget as HTMLInputElement).blur();
      }
    }}
  />

  {#if open && shown.length}
    <div
      class="absolute left-0 right-0 top-full z-50 mt-1 max-h-64 overflow-y-auto rounded-lg border border-slate-200 bg-white py-1 shadow-lg dark:border-slate-700 dark:bg-slate-800"
    >
      {#each shown as o, i (o)}
        <button
          type="button"
          onmousedown={(e) => {
            e.preventDefault();
            selectOption(o);
          }}
          class="block w-full truncate px-3 py-1.5 text-left font-mono text-sm {i ===
          highlighted
            ? 'bg-indigo-50 text-indigo-700 dark:bg-slate-700 dark:text-indigo-300'
            : 'text-slate-700 hover:bg-slate-50 dark:text-slate-200 dark:hover:bg-slate-700/60'}"
        >
          {o}
        </button>
      {/each}
      {#if filtered.length > MAX}
        <div class="px-3 py-1 text-xs text-slate-400">
          还有 {filtered.length - MAX} 项，继续输入筛选…
        </div>
      {/if}
    </div>
  {/if}
</div>

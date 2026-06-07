<script lang="ts">
  import { cls } from "../../ui";
  import {
    KEY_ACTIONS,
    keymap,
    setBinding,
    resetBinding,
    resetAll,
    eventCombo,
  } from "../../keys.svelte";

  let { onclose }: { onclose: () => void } = $props();
  let recording = $state<{ id: string; which: "long" | "short" } | null>(null);

  function onRecordKey(e: KeyboardEvent) {
    if (!recording) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === "Escape") {
      recording = null;
      return;
    }
    if (e.key === "Backspace" || e.key === "Delete") {
      setBinding(recording.id, recording.which, ""); // 清空
      recording = null;
      return;
    }
    const combo = eventCombo(e);
    if (!combo) return; // 纯修饰键，继续等
    setBinding(recording.id, recording.which, combo);
    recording = null;
  }
</script>

<svelte:window onkeydown={recording ? onRecordKey : undefined} />

<div class="fixed inset-0 z-40 flex items-center justify-center bg-black/40 p-4" role="presentation" onclick={onclose}>
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="max-h-[85vh] w-full max-w-xl overflow-y-auto rounded-xl border border-slate-200 bg-white p-5 shadow-xl dark:border-slate-700 dark:bg-slate-900"
    onclick={(e) => e.stopPropagation()}
  >
    <div class="mb-2 flex items-center justify-between">
      <h2 class="text-base font-semibold text-slate-800 dark:text-slate-100">快捷键</h2>
      <button class="{cls.btn} px-2 py-1 text-xs" onclick={resetAll}>全部重置</button>
    </div>
    <p class="mb-3 text-xs text-slate-400">
      每个操作有「长键」「短键」两套，按其一即可触发。点击键位后按下新组合修改；Esc 取消、Backspace 清空。
      中文系统下 Ctrl+Shift 可能被输入法占用，可改用短键或别的组合。
    </p>

    <div class="flex items-center gap-2 px-1 pb-1 text-[11px] uppercase tracking-wide text-slate-400">
      <span class="flex-1">操作</span>
      <span class="w-28 text-center">长键</span>
      <span class="w-24 text-center">短键</span>
      <span class="w-7"></span>
    </div>
    {#each KEY_ACTIONS as a (a.id)}
      <div class="flex items-center gap-2 rounded-lg px-1 py-1 hover:bg-slate-50 dark:hover:bg-slate-800/50">
        <span class="flex-1 truncate text-sm text-slate-700 dark:text-slate-200">{a.label}</span>
        {@render keyBtn(a.id, "long", "w-28")}
        {@render keyBtn(a.id, "short", "w-24")}
        <button class="w-7 rounded p-1 text-slate-400 transition hover:text-indigo-500" title="恢复默认" onclick={() => resetBinding(a.id)}>↺</button>
      </div>
    {/each}

    <div class="mt-4 flex justify-end">
      <button class={cls.btnPrimary} onclick={onclose}>完成</button>
    </div>
  </div>
</div>

{#snippet keyBtn(id: string, which: "long" | "short", w: string)}
  {@const rec = recording?.id === id && recording?.which === which}
  <button
    class="{w} rounded-md border px-2 py-1 text-center font-mono text-xs transition {rec
      ? 'border-indigo-500 bg-indigo-50 text-indigo-600 dark:bg-indigo-950/40 dark:text-indigo-300'
      : 'border-slate-200 text-slate-600 hover:border-indigo-400 dark:border-slate-700 dark:text-slate-300'}"
    onclick={() => (recording = { id, which })}
  >
    {rec ? "按下…" : keymap[id][which] || "—"}
  </button>
{/snippet}

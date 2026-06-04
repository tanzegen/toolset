<script lang="ts">
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";
  import Icon from "./Icon.svelte";

  let { text, size = 16 }: { text: string; size?: number } = $props();
  let copied = $state(false);
  let timer: ReturnType<typeof setTimeout>;

  async function copy() {
    if (!text) return;
    try {
      await writeText(text);
      copied = true;
      clearTimeout(timer);
      timer = setTimeout(() => (copied = false), 1200);
    } catch {
      /* 忽略：非 Tauri 环境或剪贴板不可用 */
    }
  }
</script>

<button
  onclick={copy}
  title="复制"
  aria-label="复制"
  class="shrink-0 rounded-md p-1.5 text-slate-400 transition hover:bg-slate-100 hover:text-indigo-600 dark:hover:bg-slate-800 dark:hover:text-indigo-400 {copied
    ? 'text-emerald-500'
    : ''}"
>
  <Icon name={copied ? "check" : "copy"} {size} />
</button>

<script lang="ts">
  import "../app.css";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { appState } from "../lib/state.svelte";

  let { children } = $props();

  // 主题应用到 <html>（驱动 Tailwind .dark 变体），并同步原生窗口标题栏
  // （Windows: 沉浸式深色标题栏，让顶部栏跟随夜间模式一起变黑）。非 Tauri 环境忽略。
  $effect(() => {
    const theme = appState.theme;
    document.documentElement.classList.toggle("dark", theme === "dark");
    getCurrentWindow()
      .setTheme(theme)
      .catch(() => {});
  });
</script>

{@render children()}

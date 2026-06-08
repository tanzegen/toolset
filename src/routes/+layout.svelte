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

  // 禁用浏览器/WebView 原生右键菜单（重载/返回/检查等），让它像原生桌面应用。
  // 仅在文本输入框里放行，以保留剪切/复制/粘贴的便利。应用自有的右键菜单
  // （SSH 连接、终端等）在各自元素上已自行 preventDefault，不受影响。
  function onContextMenu(e: MouseEvent) {
    const el = e.target as HTMLElement | null;
    if (el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable)) return;
    e.preventDefault();
  }
</script>

<svelte:window oncontextmenu={onContextMenu} />

{@render children()}

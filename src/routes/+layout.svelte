<script lang="ts">
  import "../app.css";
  import { appState } from "../lib/state.svelte";

  let { children } = $props();

  // 把主题应用到 <html>，驱动 Tailwind 的 .dark 变体
  $effect(() => {
    const root = document.documentElement;
    root.classList.toggle("dark", appState.theme === "dark");
  });
</script>

{@render children()}

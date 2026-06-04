// 全局 UI 状态（Svelte 5 runes）：当前工具与主题，主题持久化到 localStorage。

type Theme = "light" | "dark";

function initialTheme(): Theme {
  if (typeof localStorage !== "undefined") {
    const saved = localStorage.getItem("toolset-theme");
    if (saved === "light" || saved === "dark") return saved;
  }
  if (typeof window !== "undefined" && window.matchMedia) {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return "dark";
}

export const appState = $state({
  activeTool: "timestamp",
  theme: initialTheme() as Theme,
});

export function setTheme(t: Theme) {
  appState.theme = t;
  if (typeof localStorage !== "undefined") localStorage.setItem("toolset-theme", t);
}

export function toggleTheme() {
  setTheme(appState.theme === "dark" ? "light" : "dark");
}

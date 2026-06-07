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

function initialPinned(): string[] {
  if (typeof localStorage !== "undefined") {
    try {
      const s = localStorage.getItem("toolset-pinned");
      if (s) return JSON.parse(s) as string[];
    } catch {
      // 损坏忽略
    }
  }
  return [];
}

function initialCollapsed(): boolean {
  if (typeof localStorage !== "undefined") {
    return localStorage.getItem("toolset-sidebar") === "collapsed";
  }
  return false;
}

export const SIDEBAR_MIN = 180;
export const SIDEBAR_MAX = 420;

function initialSidebarWidth(): number {
  if (typeof localStorage !== "undefined") {
    const n = Number(localStorage.getItem("toolset-sidebar-width"));
    if (n >= SIDEBAR_MIN && n <= SIDEBAR_MAX) return n;
  }
  return 240; // 约等于原 w-60
}

export const appState = $state({
  activeTool: "timestamp",
  theme: initialTheme() as Theme,
  pinned: initialPinned(),
  sidebarCollapsed: initialCollapsed(),
  sidebarWidth: initialSidebarWidth(),
});

export function setTheme(t: Theme) {
  appState.theme = t;
  if (typeof localStorage !== "undefined") localStorage.setItem("toolset-theme", t);
}

export function toggleTheme() {
  setTheme(appState.theme === "dark" ? "light" : "dark");
}

/** 固定/取消固定一个工具（持久化）。 */
export function togglePin(id: string) {
  const i = appState.pinned.indexOf(id);
  if (i >= 0) appState.pinned.splice(i, 1);
  else appState.pinned.push(id);
  if (typeof localStorage !== "undefined") {
    localStorage.setItem("toolset-pinned", JSON.stringify(appState.pinned));
  }
}

/** 展开/收起侧边栏（持久化）。 */
export function toggleSidebar() {
  appState.sidebarCollapsed = !appState.sidebarCollapsed;
  if (typeof localStorage !== "undefined") {
    localStorage.setItem("toolset-sidebar", appState.sidebarCollapsed ? "collapsed" : "expanded");
  }
}

/** 设置侧边栏宽度（夹取 + 持久化）。 */
export function setSidebarWidth(px: number) {
  appState.sidebarWidth = Math.max(SIDEBAR_MIN, Math.min(SIDEBAR_MAX, Math.round(px)));
  if (typeof localStorage !== "undefined") {
    localStorage.setItem("toolset-sidebar-width", String(appState.sidebarWidth));
  }
}

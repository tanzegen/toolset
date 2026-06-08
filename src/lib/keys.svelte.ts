// SSH 终端快捷键管理：每个操作有「长键」「短键」两套，按其一即可触发；
// 用户可改，持久化到 localStorage。用 e.code 归一化，避开 Shift 改变字符值的问题。

export interface KeyAction {
  id: string;
  label: string;
  long: string; // 默认长键
  short: string; // 默认短键（可空）
}

export const KEY_ACTIONS: KeyAction[] = [
  // 窗口级动作：由 SshTerminal 的窗口 keydown 统一处理（开/关/切标签、搜索、缩放）。
  { id: "newTab", label: "重开最近关闭的标签", long: "Ctrl+Shift+T", short: "" },
  { id: "closeTab", label: "关闭当前标签", long: "Ctrl+Shift+W", short: "" },
  { id: "reconnect", label: "重连当前标签", long: "Ctrl+Shift+R", short: "" },
  { id: "nextTab", label: "下一个标签", long: "Ctrl+Tab", short: "" },
  { id: "prevTab", label: "上一个标签", long: "Ctrl+Shift+Tab", short: "" },
  { id: "search", label: "终端内搜索", long: "Ctrl+Shift+F", short: "Ctrl+F" },
  { id: "zoomIn", label: "增大字号", long: "Ctrl+Shift+=", short: "Ctrl+=" },
  { id: "zoomOut", label: "减小字号", long: "Ctrl+Shift+-", short: "Ctrl+-" },
  // 行编辑动作：由 TermView 翻译成 shell 行编辑序列发给远端（仅终端聚焦时生效）。
  { id: "wordLeft", label: "跳到上一个单词", long: "Ctrl+←", short: "" },
  { id: "wordRight", label: "跳到下一个单词", long: "Ctrl+→", short: "" },
  { id: "killWordBack", label: "删除上一个单词", long: "Ctrl+⌫", short: "" },
  { id: "killLine", label: "删除整行", long: "Ctrl+Shift+⌫", short: "" },
  { id: "killWordForward", label: "删除下一个单词", long: "Ctrl+⌦", short: "" },
];

type Binding = { long: string; short: string };

function load(): Record<string, Binding> {
  const map: Record<string, Binding> = {};
  for (const a of KEY_ACTIONS) map[a.id] = { long: a.long, short: a.short };
  if (typeof localStorage !== "undefined") {
    try {
      const saved = JSON.parse(localStorage.getItem("ssh-keymap") || "{}") as Record<string, Binding>;
      for (const id of Object.keys(map)) {
        if (saved[id]) {
          map[id] = { long: saved[id].long ?? map[id].long, short: saved[id].short ?? map[id].short };
        }
      }
    } catch {
      // 损坏忽略
    }
  }
  return map;
}

export const keymap = $state<Record<string, Binding>>(load());

function persist() {
  if (typeof localStorage !== "undefined") localStorage.setItem("ssh-keymap", JSON.stringify(keymap));
}

export function setBinding(id: string, which: "long" | "short", combo: string) {
  if (keymap[id]) {
    keymap[id][which] = combo;
    persist();
  }
}
export function resetBinding(id: string) {
  const def = KEY_ACTIONS.find((a) => a.id === id);
  if (def && keymap[id]) {
    keymap[id] = { long: def.long, short: def.short };
    persist();
  }
}
export function resetAll() {
  for (const a of KEY_ACTIONS) keymap[a.id] = { long: a.long, short: a.short };
  persist();
}

// —— 事件 → 组合串（用 e.code，稳定不受 Shift 影响）——
const CODE_MAP: Record<string, string> = {
  Equal: "=",
  Minus: "-",
  Tab: "Tab",
  Space: "Space",
  Backquote: "`",
  Slash: "/",
  Backslash: "\\",
  Period: ".",
  Comma: ",",
  Semicolon: ";",
  Quote: "'",
  BracketLeft: "[",
  BracketRight: "]",
  Enter: "Enter",
  Backspace: "⌫",
  Delete: "⌦",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  PageUp: "PageUp",
  PageDown: "PageDown",
};
function normKey(e: KeyboardEvent): string {
  const c = e.code;
  if (/^Key[A-Z]$/.test(c)) return c.slice(3);
  if (/^Digit\d$/.test(c)) return c.slice(5);
  if (/^F\d{1,2}$/.test(c)) return c;
  return CODE_MAP[c] ?? "";
}

export function eventCombo(e: KeyboardEvent): string {
  const key = normKey(e);
  if (!key) return ""; // 纯修饰键
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  parts.push(key);
  return parts.join("+");
}

/** 返回命中的操作 id；没命中返回 null。 */
export function matchAction(e: KeyboardEvent): string | null {
  const combo = eventCombo(e);
  if (!combo) return null;
  for (const id of Object.keys(keymap)) {
    const b = keymap[id];
    if (b.long === combo || (b.short && b.short === combo)) return id;
  }
  return null;
}

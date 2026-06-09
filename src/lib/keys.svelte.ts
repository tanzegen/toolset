// SSH 终端快捷键管理：每个操作有「长键」「短键」两套，按其一即可触发；
// 用户可改，持久化到 localStorage。用 e.code 归一化，避开 Shift 改变字符值的问题。
// 默认键位随平台而变：macOS 用 ⌘(Cmd) 系做窗口动作、⌥(Option) 系做行编辑，
// 既贴合 mac 习惯，又避开 Ctrl+方向键被「调度中心」抢占的问题。

import { IS_MAC } from "./platform";

export interface KeyAction {
  id: string;
  label: string;
  long: string; // 默认长键
  short: string; // 默认短键（可空）
  macLong?: string; // macOS 默认长键（覆盖 long）
  macShort?: string; // macOS 默认短键（覆盖 short）
}

export const KEY_ACTIONS: KeyAction[] = [
  // 窗口级动作：由 SshTerminal 的窗口 keydown 统一处理（开/关/切标签、搜索、缩放）。
  // mac 默认沿用「⇧⌘」组合，避开会被原生菜单吃掉的裸 ⌘W / ⌘Q 等。
  { id: "newTab", label: "重开最近关闭的标签", long: "Ctrl+Shift+T", short: "", macLong: "Shift+Cmd+T" },
  { id: "closeTab", label: "关闭当前标签", long: "Ctrl+Shift+W", short: "", macLong: "Shift+Cmd+W" },
  { id: "reconnect", label: "重连当前标签", long: "Ctrl+Shift+R", short: "", macLong: "Shift+Cmd+R" },
  { id: "nextTab", label: "下一个标签", long: "Ctrl+Tab", short: "" },
  { id: "prevTab", label: "上一个标签", long: "Ctrl+Shift+Tab", short: "" },
  { id: "search", label: "终端内搜索", long: "Ctrl+Shift+F", short: "Ctrl+F", macLong: "Shift+Cmd+F", macShort: "Cmd+F" },
  { id: "zoomIn", label: "增大字号", long: "Ctrl+Shift+=", short: "Ctrl+=", macLong: "Shift+Cmd+=", macShort: "Cmd+=" },
  { id: "zoomOut", label: "减小字号", long: "Ctrl+Shift+-", short: "Ctrl+-", macLong: "Shift+Cmd+-", macShort: "Cmd+-" },
  // 行编辑动作：由 TermView 翻译成 shell 行编辑序列发给远端（仅终端聚焦时生效）。
  // mac 默认用 ⌥(Option) 跳词/删词，对齐系统终端习惯；整行用 ⌘⌫。
  { id: "wordLeft", label: "跳到上一个单词", long: "Ctrl+←", short: "", macLong: "Alt+←" },
  { id: "wordRight", label: "跳到下一个单词", long: "Ctrl+→", short: "", macLong: "Alt+→" },
  { id: "killWordBack", label: "删除上一个单词", long: "Ctrl+⌫", short: "", macLong: "Alt+⌫" },
  { id: "killLine", label: "删除整行", long: "Ctrl+Shift+⌫", short: "", macLong: "Cmd+⌫" },
  { id: "killWordForward", label: "删除下一个单词", long: "Ctrl+⌦", short: "", macLong: "Alt+⌦" },
];

type Binding = { long: string; short: string };

// 取某动作在当前平台下的默认绑定（mac 用 macLong/macShort 覆盖）。
function defaults(a: KeyAction): Binding {
  if (IS_MAC) return { long: a.macLong ?? a.long, short: a.macShort ?? a.short };
  return { long: a.long, short: a.short };
}

function load(): Record<string, Binding> {
  const map: Record<string, Binding> = {};
  for (const a of KEY_ACTIONS) map[a.id] = defaults(a);
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
    keymap[id] = defaults(def);
    persist();
  }
}
export function resetAll() {
  for (const a of KEY_ACTIONS) keymap[a.id] = defaults(a);
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
  // 顺序对齐 mac 习惯 ⌃⌥⇧⌘：Ctrl、Alt(Option)、Shift、Cmd(Meta)。
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Cmd");
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

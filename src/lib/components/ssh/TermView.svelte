<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { SearchAddon } from "@xterm/addon-search";
  import { Channel } from "@tauri-apps/api/core";
  import { writeText, readText } from "@tauri-apps/plugin-clipboard-manager";
  import { TrzszFilter } from "trzsz";
  import "@xterm/xterm/css/xterm.css";
  import { ssh, b64ToBytes, VAULT_LOCKED, type SshFrame } from "../../ssh";

  let {
    connId,
    fontSize = 13,
    relogin = 0,
    active = false,
    onstatus,
    onlocked,
  }: {
    connId: string;
    fontSize?: number;
    relogin?: number;
    active?: boolean;
    onstatus?: (s: string) => void;
    onlocked?: () => void;
  } = $props();

  let host = $state<HTMLDivElement>();
  let term: Terminal | undefined;
  let fit: FitAddon | undefined;
  let searchAddon: SearchAddon | undefined;
  let trzsz: TrzszFilter | undefined;
  let sessionId = "";
  let ro: ResizeObserver | undefined;
  let lastRelogin = relogin;
  let dragging = $state(false);

  // 重连 / 退避
  let gen = 0; // 会话代号，旧会话的 Channel 消息据此忽略
  let disposed = false;
  let backoffStep = 0;
  let reconnectTimer: ReturnType<typeof setTimeout> | undefined;

  // 搜索
  let searchOpen = $state(false);
  let searchQuery = $state("");
  let searchInput = $state<HTMLInputElement>();

  function setStatus(s: string) {
    onstatus?.(s);
  }

  function writeOut(out: string | ArrayBuffer | Uint8Array | Blob) {
    if (typeof out === "string" || out instanceof Uint8Array) term?.write(out);
    else if (out instanceof ArrayBuffer) term?.write(new Uint8Array(out));
    else if (out instanceof Blob) out.arrayBuffer().then((ab) => term?.write(new Uint8Array(ab)));
  }

  // 拖拽上传：把拖入的文件交给 trzsz（会自动在远端触发 trz）。需在 shell 提示符处拖放。
  function onDrop(e: DragEvent) {
    e.preventDefault();
    dragging = false;
    const items = e.dataTransfer?.items;
    if (!trzsz || !items || !items.length) return;
    Promise.resolve(trzsz.uploadFiles(items)).catch((err) =>
      term?.writeln(`\r\n\x1b[31m[上传失败] ${err}\x1b[0m`),
    );
  }

  function clearReconnect() {
    if (reconnectTimer) {
      clearTimeout(reconnectTimer);
      reconnectTimer = undefined;
    }
  }

  // 指数退避：1,2,4,8,15,30s，封顶 30s
  function scheduleReconnect() {
    if (disposed) return;
    const delays = [1000, 2000, 4000, 8000, 15000, 30000];
    const delay = delays[Math.min(backoffStep, delays.length - 1)];
    backoffStep++;
    setStatus("reconnecting");
    term?.writeln(`\x1b[90m${delay / 1000}s 后自动重连…（点「重连」可立即重试）\x1b[0m`);
    reconnectTimer = setTimeout(() => {
      reconnectTimer = undefined;
      connect();
    }, delay);
  }

  async function connect() {
    if (!term || disposed) return;
    const myGen = ++gen;
    setStatus("connecting");
    term.writeln("\x1b[90m正在连接…\x1b[0m");
    const channel = new Channel<SshFrame>();
    channel.onmessage = (f) => {
      if (myGen !== gen || disposed) return; // 旧会话 / 已销毁，忽略
      if (f.kind === "data") {
        trzsz?.processServerOutput(b64ToBytes(f.data));
      } else if (f.kind === "status") {
        if (f.state === "connected") {
          setStatus("connected");
        } else if (f.state === "exited") {
          setStatus("exited");
          term!.writeln("\r\n\x1b[90m[会话已结束]\x1b[0m");
        } else if (f.state === "closed") {
          term!.writeln("\r\n\x1b[33m[连接已断开]\x1b[0m");
          scheduleReconnect();
        }
      }
    };
    try {
      sessionId = await ssh.connect(connId, channel);
      backoffStep = 0; // 连上即重置退避
      if (term) ssh.resize(sessionId, term.cols, term.rows);
      if (active) term.focus();
    } catch (e) {
      const msg = String(e);
      if (msg.includes(VAULT_LOCKED)) {
        setStatus("locked");
        term.writeln("\x1b[33m[需要先解锁主密码]\x1b[0m");
        onlocked?.();
      } else if (msg.includes("认证")) {
        // 认证问题不会自己好，停止重连，等用户改配置后手动重连
        setStatus("error");
        term.writeln(`\x1b[31m[${msg}]\x1b[0m`);
      } else {
        setStatus("error");
        term.writeln(`\x1b[31m[连接失败] ${msg}\x1b[0m`);
        // 仅在「重连过程中」继续退避重试；首次连接失败不自动重连
        if (backoffStep > 0) scheduleReconnect();
      }
    }
  }

  /** 供父组件调用：立即断开并重连（重置退避）。 */
  export async function reconnect() {
    clearReconnect();
    backoffStep = 0;
    if (sessionId) {
      ssh.close(sessionId);
      sessionId = "";
    }
    term?.reset();
    await connect();
  }

  function toggleSearch() {
    searchOpen = !searchOpen;
    if (searchOpen) setTimeout(() => searchInput?.select(), 0);
    else {
      searchAddon?.clearDecorations();
      term?.focus();
    }
  }

  // 右键：有选区则复制，否则粘贴（PuTTY / Windows Terminal 习惯）
  async function onContextMenu(e: MouseEvent) {
    e.preventDefault();
    const sel = term?.getSelection();
    if (sel) {
      writeText(sel);
      term?.clearSelection();
      return;
    }
    try {
      const t = await readText();
      if (t) trzsz?.processTerminalInput(t);
    } catch {
      // 剪贴板不可读时忽略
    }
  }

  // relogin 计数变化 → 重连
  $effect(() => {
    if (relogin !== lastRelogin) {
      lastRelogin = relogin;
      reconnect();
    }
  });

  // 切到该标签（变为可见）时自动聚焦
  $effect(() => {
    if (active && term) term.focus();
  });

  // 字号变化即时生效。无条件先读 fontSize 以确保被追踪——
  // 否则首次运行时 term 尚未创建，fontSize 不会被登记为依赖，后续改动不触发重跑。
  $effect(() => {
    const fs = fontSize;
    if (term) {
      term.options.fontSize = fs;
      fit?.fit();
      if (sessionId) ssh.resize(sessionId, term.cols, term.rows);
    }
  });

  onMount(() => {
    term = new Terminal({
      fontSize,
      fontFamily: 'Consolas, "Cascadia Mono", "JetBrains Mono", Menlo, monospace',
      cursorBlink: true,
      scrollback: 5000,
      theme: { background: "#0b1020", foreground: "#d6deeb", cursor: "#7dd3fc" },
    });
    fit = new FitAddon();
    searchAddon = new SearchAddon();
    term.loadAddon(fit);
    term.loadAddon(searchAddon);
    term.open(host!);
    fit.fit();

    // trzsz 过滤器：拦截 trz/tsz 做文件传输，其余原样透传。
    trzsz = new TrzszFilter({
      writeToTerminal: writeOut,
      sendToServer: (input) => {
        if (sessionId) ssh.write(sessionId, input);
      },
      terminalColumns: term.cols,
    });
    term.onData((d) => trzsz?.processTerminalInput(d));
    host!.addEventListener("contextmenu", onContextMenu);

    // 复制/粘贴/搜索快捷键：
    // - Ctrl+C：有选区→复制并清选区；无选区→放行作中断(SIGINT)。
    // - Ctrl+V：交给 xterm 原生 paste（避免双重粘贴）。
    // - Ctrl+Shift+F：搜索。Ctrl+Shift+T/R：交给父级窗口（新标签/重连）。
    term.attachCustomKeyEventHandler((e) => {
      if (e.type !== "keydown" || !e.ctrlKey) return true;
      const k = e.key.toLowerCase();
      if (k === "c") {
        const sel = term!.getSelection();
        if (e.shiftKey || sel) {
          if (sel) {
            writeText(sel);
            term!.clearSelection();
          }
          return false;
        }
        return true;
      }
      if (k === "v") return false;
      if (k === "f") {
        // Ctrl+F（及 Ctrl+Shift+F）打开搜索。中文系统里 Ctrl+Shift 常被输入法切换占用，
        // 所以主用 Ctrl+F；preventDefault 阻止 WebView 自带的页面查找。
        e.preventDefault();
        toggleSearch();
        return false;
      }
      if (e.shiftKey && (k === "t" || k === "r")) return false;
      return true;
    });

    ro = new ResizeObserver(() => {
      try {
        fit?.fit();
        if (term) trzsz?.setTerminalColumns(term.cols);
        if (sessionId && term) ssh.resize(sessionId, term.cols, term.rows);
      } catch {
        // 容器临时为 0 尺寸（被隐藏）时忽略
      }
    });
    ro.observe(host!);

    connect();
  });

  onDestroy(() => {
    disposed = true;
    clearReconnect();
    ro?.disconnect();
    host?.removeEventListener("contextmenu", onContextMenu);
    if (sessionId) ssh.close(sessionId);
    term?.dispose();
  });
</script>

<div
  class="relative h-full w-full"
  role="presentation"
  ondragover={(e) => {
    e.preventDefault();
    dragging = true;
  }}
  ondragleave={(e) => {
    if (!e.relatedTarget || !(e.currentTarget as Node).contains(e.relatedTarget as Node)) dragging = false;
  }}
  ondrop={onDrop}
>
  <div bind:this={host} class="h-full w-full overflow-hidden rounded-lg" style="background:#0b1020"></div>

  {#if searchOpen}
    <div class="absolute right-2 top-2 z-20 flex items-center gap-1 rounded-md bg-slate-800/95 px-1.5 py-1 shadow-lg ring-1 ring-white/10">
      <input
        bind:this={searchInput}
        bind:value={searchQuery}
        placeholder="搜索…"
        class="w-40 bg-transparent px-1 text-xs text-slate-100 outline-none placeholder:text-slate-500"
        oninput={() => searchQuery && searchAddon?.findNext(searchQuery, { incremental: true })}
        onkeydown={(e) => {
          if (e.ctrlKey && e.key.toLowerCase() === "f") {
            e.preventDefault();
            searchInput?.select(); // 已打开时再按 Ctrl+F 重新聚焦/全选
          } else if (e.key === "Enter") {
            e.preventDefault();
            if (e.shiftKey) searchAddon?.findPrevious(searchQuery);
            else searchAddon?.findNext(searchQuery);
          } else if (e.key === "Escape") {
            e.preventDefault();
            toggleSearch();
          }
        }}
      />
      <button class="rounded px-1 text-xs text-slate-300 hover:bg-white/10" title="上一个 (Shift+Enter)" onclick={() => searchAddon?.findPrevious(searchQuery)}>↑</button>
      <button class="rounded px-1 text-xs text-slate-300 hover:bg-white/10" title="下一个 (Enter)" onclick={() => searchAddon?.findNext(searchQuery)}>↓</button>
      <button class="rounded px-1 text-xs text-slate-300 hover:bg-white/10" title="关闭 (Esc)" onclick={toggleSearch}>✕</button>
    </div>
  {/if}

  {#if dragging}
    <div
      class="pointer-events-none absolute inset-0 flex items-center justify-center rounded-lg border-2 border-dashed border-indigo-400 bg-indigo-500/15 text-sm font-medium text-indigo-100"
    >
      松开以上传到当前目录（自动触发 trz）
    </div>
  {/if}
</div>

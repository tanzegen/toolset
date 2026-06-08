<script lang="ts">
  import { onDestroy } from "svelte";
  import { Channel } from "@tauri-apps/api/core";
  import { cls } from "../ui";
  import { persist } from "../persist.svelte";
  import Icon from "../components/Icon.svelte";
  import Stepper from "../components/Stepper.svelte";
  import { net, type WsFrame } from "../net";

  type KV = { key: string; value: string; on: boolean };
  type Msg = { dir: "in" | "out" | "sys"; text: boolean; data: string; at: number };
  const blank = (): KV => ({ key: "", value: "", on: true });

  let url = $state("wss://echo.websocket.events");
  let headers = $state<KV[]>([blank()]);
  let showHeaders = $state(false);
  // 连接事件：连上后按顺序自动发送的初始化消息（鉴权/订阅等）
  let showInit = $state(false);
  let initMsgs = $state<{ text: string; on: boolean }[]>([{ text: "", on: true }]);
  let sendMode = $state<"text" | "binary">("text");
  let input = $state("");
  let autoReconnect = $state(false);
  let savedUrls = $state<string[]>([]);

  // 心跳：保活空闲连接。ping=协议级 Ping 帧；text=自定义应用层消息。
  let hbOn = $state(false);
  let hbInterval = $state(30);
  let hbMode = $state<"ping" | "text">("ping");
  let hbText = $state('{"type":"ping"}');
  let hbLast = $state(0);

  let messages = $state<Msg[]>([]);
  let status = $state<"closed" | "connecting" | "open" | "error">("closed");
  let sessionId = "";
  let manualClose = false;
  let logEl = $state<HTMLDivElement>();

  persist("ws-client", {
    save: () => ({ url, headers, initMsgs, autoReconnect, savedUrls, sendMode, hbOn, hbInterval, hbMode, hbText }),
    load: (s) => {
      url = s.url ?? url;
      headers = s.headers ?? headers;
      initMsgs = s.initMsgs ?? initMsgs;
      autoReconnect = s.autoReconnect ?? autoReconnect;
      savedUrls = s.savedUrls ?? savedUrls;
      sendMode = s.sendMode ?? sendMode;
      hbOn = s.hbOn ?? hbOn;
      hbInterval = s.hbInterval ?? hbInterval;
      hbMode = s.hbMode ?? hbMode;
      hbText = s.hbText ?? hbText;
    },
  });

  // 心跳定时器：仅在已连接且开启时运行；状态/开关/间隔变化即重建（cleanup 清旧定时器）。
  $effect(() => {
    if (status !== "open" || !hbOn) return;
    const sec = Math.max(1, hbInterval);
    const id = setInterval(() => {
      if (status !== "open" || !sessionId) return;
      if (hbMode === "ping") net.wsPing(sessionId).catch(() => {});
      else net.wsSend(sessionId, true, hbText).catch(() => {});
      hbLast = Date.now();
    }, sec * 1000);
    return () => clearInterval(id);
  });

  function grow(rows: KV[]) {
    const last = rows[rows.length - 1];
    if (!last || last.key || last.value) rows.push(blank());
  }
  function growInit() {
    const last = initMsgs[initMsgs.length - 1];
    if (!last || last.text) initMsgs.push({ text: "", on: true });
  }
  function addSys(m: string) {
    messages.push({ dir: "sys", text: true, data: m, at: Date.now() });
  }

  // 新消息时滚到底部
  $effect(() => {
    void messages.length;
    if (logEl) logEl.scrollTop = logEl.scrollHeight;
  });

  async function connect() {
    if (status === "open" || status === "connecting") return;
    manualClose = false;
    status = "connecting";
    addSys(`连接 ${url} …`);
    const ch = new Channel<WsFrame>();
    ch.onmessage = (f) => {
      if (f.kind === "status") {
        if (f.state === "open") {
          status = "open";
          addSys("已连接");
          // 连接事件：按顺序自动发送初始化消息
          for (const m of initMsgs) {
            if (m.on && m.text.trim()) {
              net.wsSend(sessionId, true, m.text).catch(() => {});
              messages.push({ dir: "out", text: true, data: m.text, at: Date.now() });
            }
          }
        } else if (f.state === "closed") {
          status = "closed";
          sessionId = "";
          addSys("连接已关闭");
          if (autoReconnect && !manualClose) setTimeout(connect, 2000);
        } else if (f.state === "error") {
          status = "error";
          addSys("错误：" + f.msg);
        }
      } else {
        messages.push({ dir: "in", text: f.text, data: f.data, at: Date.now() });
      }
    };
    try {
      sessionId = await net.wsConnect(
        url,
        headers.filter((h) => h.on && h.key.trim()).map((h) => ({ key: h.key, value: h.value })),
        ch,
      );
    } catch (e) {
      status = "error";
      addSys("连接失败：" + String(e));
    }
  }

  function disconnect() {
    manualClose = true;
    if (sessionId) net.wsClose(sessionId);
  }

  async function send() {
    if (status !== "open" || !sessionId || !input) return;
    try {
      await net.wsSend(sessionId, sendMode === "text", input);
      messages.push({ dir: "out", text: sendMode === "text", data: input, at: Date.now() });
      input = "";
    } catch (e) {
      addSys("发送失败：" + String(e));
    }
  }

  function saveUrl() {
    if (url.trim() && !savedUrls.includes(url.trim())) savedUrls.unshift(url.trim());
  }

  onDestroy(() => {
    if (sessionId) net.wsClose(sessionId);
  });

  const statusBadge = $derived(
    status === "open"
      ? "bg-emerald-50 text-emerald-600 dark:bg-emerald-950/40 dark:text-emerald-300"
      : status === "connecting"
        ? "bg-amber-50 text-amber-600 dark:bg-amber-950/40 dark:text-amber-300"
        : status === "error"
          ? "bg-red-50 text-red-600 dark:bg-red-950/40 dark:text-red-300"
          : "bg-slate-100 text-slate-500 dark:bg-slate-800 dark:text-slate-400",
  );
  const statusText = $derived(
    status === "open" ? "已连接" : status === "connecting" ? "连接中" : status === "error" ? "错误" : "未连接",
  );
  const fmt = (t: number) => new Date(t).toLocaleTimeString();
</script>

<div class="flex h-full flex-col overflow-hidden">
  <!-- 顶部：URL + 连接 -->
  <div class="flex flex-wrap items-center gap-1.5 border-b border-slate-200 p-2 dark:border-slate-800">
    <span class="rounded px-2 py-1 text-[11px] {statusBadge}">{statusText}</span>
    <input
      bind:value={url}
      list="ws-saved"
      onkeydown={(e) => e.key === "Enter" && (status === "open" ? undefined : connect())}
      placeholder="ws:// 或 wss:// 地址"
      class="{cls.field} min-w-[220px] flex-1 py-1.5 font-mono text-sm"
      disabled={status === "open" || status === "connecting"}
    />
    <datalist id="ws-saved">{#each savedUrls as u (u)}<option value={u}></option>{/each}</datalist>
    {#if status === "open" || status === "connecting"}
      <button class="{cls.btn} shrink-0 px-4 py-1.5" onclick={disconnect}>断开</button>
    {:else}
      <button class="{cls.btnPrimary} shrink-0 px-4 py-1.5" onclick={connect}><Icon name="zap" size={15} />连接</button>
    {/if}
    <button class="{cls.btn} shrink-0 px-2 py-1.5 text-xs {showHeaders ? 'text-indigo-600 dark:text-indigo-300' : ''}" onclick={() => (showHeaders = !showHeaders)} title="握手请求头">头</button>
    <button class="{cls.btn} shrink-0 px-2 py-1.5 text-xs {showInit ? 'text-indigo-600 dark:text-indigo-300' : ''}" onclick={() => (showInit = !showInit)} title="连接后自动发送的消息">事件</button>
    <button class="{cls.btn} shrink-0 px-2 py-1.5 text-xs" onclick={saveUrl} title="保存此地址"><Icon name="pin" size={14} /></button>
  </div>

  {#if showHeaders}
    <div class="border-b border-slate-200 p-3 dark:border-slate-800">
      <p class="mb-1 text-xs text-slate-400">连接握手时附带的请求头（连接前设置）：</p>
      <div class="space-y-1">
        {#each headers as row, i (i)}
          <div class="flex items-center gap-1.5">
            <input type="checkbox" class="accent-indigo-600" bind:checked={row.on} />
            <input class="{cls.field} min-w-0 flex-1 py-1 font-mono text-xs" placeholder="键" bind:value={row.key} oninput={() => grow(headers)} />
            <input class="{cls.field} min-w-0 flex-1 py-1 font-mono text-xs" placeholder="值" bind:value={row.value} oninput={() => grow(headers)} />
            <button class="rounded p-1 text-slate-400 hover:text-red-500" title="删除" aria-label="删除" onclick={() => { headers.splice(i, 1); if (!headers.length) headers.push(blank()); }}>✕</button>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  {#if showInit}
    <div class="border-b border-slate-200 p-3 dark:border-slate-800">
      <p class="mb-1 text-xs text-slate-400">连接成功后按顺序自动发送（如鉴权 / 订阅消息）：</p>
      <div class="space-y-1">
        {#each initMsgs as m, i (i)}
          <div class="flex items-center gap-1.5">
            <input type="checkbox" class="accent-indigo-600" bind:checked={m.on} />
            <span class="w-4 shrink-0 text-center text-[10px] text-slate-400">{i + 1}</span>
            <input class="{cls.field} min-w-0 flex-1 py-1 font-mono text-xs" placeholder={'如 {"type":"subscribe","channel":"..."}'} bind:value={m.text} oninput={growInit} />
            <button class="rounded p-1 text-slate-400 hover:text-red-500" title="删除" aria-label="删除" onclick={() => { initMsgs.splice(i, 1); if (!initMsgs.length) initMsgs.push({ text: '', on: true }); }}>✕</button>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  <!-- 消息时间线 -->
  <div bind:this={logEl} class="min-h-0 flex-1 space-y-1 overflow-y-auto bg-slate-50/50 p-3 dark:bg-slate-900/30">
    {#each messages as m, i (i)}
      <div class="flex gap-2 text-xs">
        <span class="shrink-0 text-[10px] text-slate-400">{fmt(m.at)}</span>
        {#if m.dir === "sys"}
          <span class="italic text-slate-400">{m.data}</span>
        {:else}
          <span class="shrink-0 font-semibold {m.dir === 'in' ? 'text-emerald-600 dark:text-emerald-400' : 'text-indigo-600 dark:text-indigo-400'}">{m.dir === "in" ? "↓收" : "↑发"}{m.text ? "" : "·bin"}</span>
          <span class="min-w-0 flex-1 select-text break-all font-mono text-slate-700 dark:text-slate-200">{m.data}</span>
        {/if}
      </div>
    {/each}
    {#if messages.length === 0}
      <div class="flex h-full items-center justify-center text-sm text-slate-400">连接后在下方发送消息。可设握手请求头，无浏览器限制。</div>
    {/if}
  </div>

  <!-- 发送区 -->
  <div class="border-t border-slate-200 p-2 dark:border-slate-800">
    <div class="mb-1.5 flex flex-wrap items-center gap-x-3 gap-y-1.5 text-xs">
      <div class="flex gap-1">
        <button class="rounded px-2 py-0.5 {sendMode === 'text' ? 'bg-indigo-600 text-white' : 'border border-slate-200 text-slate-600 dark:border-slate-700 dark:text-slate-300'}" onclick={() => (sendMode = "text")}>文本</button>
        <button class="rounded px-2 py-0.5 {sendMode === 'binary' ? 'bg-indigo-600 text-white' : 'border border-slate-200 text-slate-600 dark:border-slate-700 dark:text-slate-300'}" onclick={() => (sendMode = "binary")}>二进制(hex)</button>
      </div>
      <label class="flex items-center gap-1 text-slate-500"><input type="checkbox" class="accent-indigo-600" bind:checked={autoReconnect} /> 自动重连</label>

      <!-- 心跳保活 -->
      <label class="flex items-center gap-1 text-slate-500"><input type="checkbox" class="accent-indigo-600" bind:checked={hbOn} /> 心跳</label>
      {#if hbOn}
        <span class="flex items-center gap-1 text-slate-500">每<Stepper bind:value={hbInterval} min={1} step={5} width="w-10" />秒</span>
        <div class="flex gap-1">
          <button class="rounded px-2 py-0.5 {hbMode === 'ping' ? 'bg-indigo-600 text-white' : 'border border-slate-200 text-slate-600 dark:border-slate-700 dark:text-slate-300'}" onclick={() => (hbMode = 'ping')} title="协议级 Ping 帧（推荐）">Ping帧</button>
          <button class="rounded px-2 py-0.5 {hbMode === 'text' ? 'bg-indigo-600 text-white' : 'border border-slate-200 text-slate-600 dark:border-slate-700 dark:text-slate-300'}" onclick={() => (hbMode = 'text')} title="定时发送自定义消息">消息</button>
        </div>
        {#if hbMode === 'text'}
          <input bind:value={hbText} placeholder="心跳消息" class="{cls.field} w-40 py-0.5 font-mono text-xs" />
        {/if}
        {#if hbLast}<span class="text-[10px] text-slate-400">♥ {new Date(hbLast).toLocaleTimeString()}</span>{/if}
      {/if}

      <button class="ml-auto {cls.btn} px-2 py-0.5 text-xs" onclick={() => (messages = [])}>清空</button>
    </div>
    <div class="flex items-end gap-1.5">
      <textarea
        bind:value={input}
        rows="2"
        onkeydown={(e) => { if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) { e.preventDefault(); send(); } }}
        placeholder={sendMode === "text" ? "消息内容（Ctrl+Enter 发送）" : "hex，如 48656c6c6f（Ctrl+Enter 发送）"}
        class="{cls.field} min-w-0 flex-1 font-mono text-xs"
      ></textarea>
      <button class="{cls.btnPrimary} shrink-0 px-4 py-2" onclick={send} disabled={status !== 'open' || !input}><Icon name="send" size={15} />发送</button>
    </div>
  </div>
</div>

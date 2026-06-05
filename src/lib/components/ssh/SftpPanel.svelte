<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { Channel } from "@tauri-apps/api/core";
  import { save, open } from "@tauri-apps/plugin-dialog";
  import Icon from "../Icon.svelte";
  import { cls } from "../../ui";
  import { ssh, bytesToBase64, VAULT_LOCKED, type SftpEntry, type TransferFrame } from "../../ssh";

  let {
    connId,
    onlocked,
  }: { connId: string; onlocked?: () => void } = $props();

  let sftpId = "";
  let cwd = $state("/");
  let pathInput = $state("/"); // 地址栏可编辑内容；导航成功后同步为 cwd
  let pathInputEl = $state<HTMLInputElement>();
  let entries = $state<SftpEntry[]>([]);
  let loading = $state(false);
  let error = $state("");
  let transfer = $state<{ name: string; transferred: number; total: number; dir: "up" | "down" } | null>(null);
  let dragging = $state(false);

  function joinPath(dir: string, name: string): string {
    return dir.endsWith("/") ? dir + name : `${dir}/${name}`;
  }
  function parentOf(p: string): string {
    if (p === "/" || p === "") return "/";
    const t = p.replace(/\/+$/, "");
    const i = t.lastIndexOf("/");
    return i <= 0 ? "/" : t.slice(0, i);
  }
  function baseName(p: string): string {
    return p.split(/[\\/]/).pop() || p;
  }
  function fmtSize(n: number): string {
    if (n < 1024) return `${n} B`;
    const u = ["KB", "MB", "GB", "TB"];
    let v = n / 1024;
    let i = 0;
    while (v >= 1024 && i < u.length - 1) {
      v /= 1024;
      i++;
    }
    return `${v.toFixed(1)} ${u[i]}`;
  }
  function fmtTime(sec: number): string {
    if (!sec) return "";
    return new Date(sec * 1000).toLocaleString();
  }

  async function list(path: string) {
    if (!sftpId) return;
    loading = true;
    error = "";
    try {
      entries = await ssh.sftpList(sftpId, path);
      cwd = path;
      pathInput = path; // 同步地址栏，保证就地导航
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function init() {
    try {
      sftpId = await ssh.sftpOpen(connId);
      const home = await ssh.sftpHome(sftpId).catch(() => "/");
      await list(home || "/");
    } catch (e) {
      const msg = String(e);
      if (msg.includes(VAULT_LOCKED)) onlocked?.();
      else error = msg;
    }
  }

  onMount(() => {
    init();
    // 聚焦地址栏：避免焦点停留在打开本面板的「文件」按钮上——否则按回车会激活该按钮、
    // 又开一个 SFTP 标签（用户报告的“输入无效 + 开新标签”正是此因）。
    setTimeout(() => pathInputEl?.focus(), 0);
  });
  onDestroy(() => {
    if (sftpId) ssh.sftpClose(sftpId);
  });

  function enter(e: SftpEntry) {
    if (e.isDir) list(joinPath(cwd, e.name));
  }

  async function doDownload(e: SftpEntry) {
    const local = await save({ defaultPath: e.name });
    if (!local) return;
    const remote = joinPath(cwd, e.name);
    transfer = { name: e.name, transferred: 0, total: e.size, dir: "down" };
    const ch = new Channel<TransferFrame>();
    ch.onmessage = (f) => {
      if (f.kind === "progress" && transfer) {
        transfer.transferred = f.transferred;
        transfer.total = f.total;
      }
    };
    try {
      await ssh.sftpDownload(sftpId, remote, local, ch);
    } catch (err) {
      error = String(err);
    } finally {
      transfer = null;
    }
  }

  async function doUpload() {
    const local = await open({ multiple: false });
    if (typeof local !== "string") return;
    const name = baseName(local);
    const remote = joinPath(cwd, name);
    transfer = { name, transferred: 0, total: 0, dir: "up" };
    const ch = new Channel<TransferFrame>();
    ch.onmessage = (f) => {
      if (f.kind === "progress" && transfer) {
        transfer.transferred = f.transferred;
        transfer.total = f.total;
      }
    };
    try {
      await ssh.sftpUpload(sftpId, local, remote, ch);
      await list(cwd);
    } catch (err) {
      error = String(err);
    } finally {
      transfer = null;
    }
  }

  async function doDelete(e: SftpEntry) {
    if (!confirm(`删除 ${e.isDir ? "目录" : "文件"}「${e.name}」？`)) return;
    const p = joinPath(cwd, e.name);
    try {
      if (e.isDir) await ssh.sftpRmdir(sftpId, p);
      else await ssh.sftpRemove(sftpId, p);
      await list(cwd);
    } catch (err) {
      error = String(err);
    }
  }

  // 拖拽上传：读取拖入的 File 字节，逐个上传到当前目录
  async function uploadOne(file: File) {
    const buf = new Uint8Array(await file.arrayBuffer());
    transfer = { name: file.name, transferred: 0, total: file.size, dir: "up" };
    await ssh.sftpUploadBytes(sftpId, joinPath(cwd, file.name), bytesToBase64(buf));
    transfer = { name: file.name, transferred: file.size, total: file.size, dir: "up" };
  }
  async function onDrop(e: DragEvent) {
    e.preventDefault();
    dragging = false;
    const files = e.dataTransfer?.files;
    if (!sftpId || !files || !files.length) return;
    try {
      for (const f of Array.from(files)) await uploadOne(f);
      await list(cwd);
    } catch (err) {
      error = String(err);
    } finally {
      transfer = null;
    }
  }

  async function doMkdir() {
    const name = prompt("新目录名");
    if (!name) return;
    try {
      await ssh.sftpMkdir(sftpId, joinPath(cwd, name));
      await list(cwd);
    } catch (err) {
      error = String(err);
    }
  }

  const pct = $derived(transfer && transfer.total ? Math.round((transfer.transferred / transfer.total) * 100) : 0);
</script>

<div
  class="relative flex h-full flex-col bg-white dark:bg-slate-900"
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
  <!-- 工具栏 + 路径 -->
  <div class="flex items-center gap-2 border-b border-slate-200 px-3 py-2 dark:border-slate-800">
    <button class="{cls.btn} px-2 py-1" title="上级" onclick={() => list(parentOf(cwd))}>↑</button>
    <button class="{cls.btn} px-2 py-1" title="刷新" onclick={() => list(cwd)}><Icon name="refresh" size={14} /></button>
    <input
      class="{cls.field} flex-1 font-mono text-xs"
      bind:this={pathInputEl}
      bind:value={pathInput}
      placeholder="输入路径后回车或点「前往」，在本面板内跳转"
      onkeydown={(e) => {
        if (e.key === "Enter" && !e.isComposing) {
          e.preventDefault();
          e.stopPropagation();
          list(pathInput.trim() || "/");
        }
      }}
    />
    <button class="{cls.btn} px-2 py-1 text-xs" title="跳转到该目录" onclick={() => list(pathInput.trim() || "/")}>前往</button>
    <button class="{cls.btn} px-2 py-1 text-xs" onclick={doMkdir}>新建目录</button>
    <button class="{cls.btnPrimary} px-2 py-1 text-xs" onclick={doUpload}><Icon name="upload" size={14} /> 上传</button>
  </div>

  {#if error}
    <p class="border-b border-red-100 bg-red-50 px-3 py-1.5 text-xs text-red-600 dark:border-red-950 dark:bg-red-950/30 dark:text-red-400">{error}</p>
  {/if}

  <!-- 列表 -->
  <div class="min-h-0 flex-1 overflow-y-auto">
    {#if loading}
      <p class="p-4 text-center text-xs text-slate-400">加载中…</p>
    {:else}
      {#each entries as e (e.name)}
        <div class="group flex items-center gap-2 border-b border-slate-50 px-3 py-1.5 text-sm hover:bg-slate-50 dark:border-slate-800/50 dark:hover:bg-slate-800/40">
          <button
            class="flex min-w-0 flex-1 items-center gap-2 text-left"
            title={e.isDir ? "双击进入" : "双击下载"}
            ondblclick={() => (e.isDir ? enter(e) : doDownload(e))}
            onclick={() => e.isDir && enter(e)}
          >
            <Icon name={e.isDir ? "folder" : "filecode"} size={15} class={e.isDir ? "text-amber-500" : "text-slate-400"} />
            <span class="truncate {e.isDir ? 'text-slate-700 dark:text-slate-200' : 'text-slate-600 dark:text-slate-300'}">{e.name}</span>
            {#if e.isLink}<span class="text-[10px] text-slate-400">↗</span>{/if}
          </button>
          <span class="w-20 shrink-0 text-right font-mono text-xs text-slate-400">{e.isDir ? "" : fmtSize(e.size)}</span>
          <span class="hidden w-36 shrink-0 text-right text-xs text-slate-400 lg:block">{fmtTime(e.mtime)}</span>
          <div class="flex w-16 shrink-0 items-center justify-end gap-0.5 opacity-0 transition group-hover:opacity-100">
            {#if !e.isDir}
              <button class="rounded p-1 hover:bg-slate-200 dark:hover:bg-slate-700" title="下载" onclick={() => doDownload(e)}><Icon name="download" size={13} /></button>
            {/if}
            <button class="rounded p-1 hover:bg-slate-200 dark:hover:bg-slate-700" title="删除" onclick={() => doDelete(e)}><Icon name="trash" size={13} /></button>
          </div>
        </div>
      {/each}
      {#if entries.length === 0}
        <p class="p-4 text-center text-xs text-slate-400">空目录</p>
      {/if}
    {/if}
  </div>

  <!-- 传输进度 -->
  {#if transfer}
    <div class="border-t border-slate-200 px-3 py-2 dark:border-slate-800">
      <div class="mb-1 flex items-center justify-between text-xs text-slate-500">
        <span class="truncate">{transfer.dir === "up" ? "上传" : "下载"} {transfer.name}</span>
        <span class="font-mono">{fmtSize(transfer.transferred)}{transfer.total ? ` / ${fmtSize(transfer.total)}` : ""}{transfer.total ? ` (${pct}%)` : ""}</span>
      </div>
      <div class="h-1.5 w-full overflow-hidden rounded bg-slate-200 dark:bg-slate-700">
        <div class="h-full bg-indigo-500 transition-all" style="width:{transfer.total ? pct : 50}%"></div>
      </div>
    </div>
  {/if}

  {#if dragging}
    <div
      class="pointer-events-none absolute inset-0 z-10 flex items-center justify-center border-2 border-dashed border-indigo-400 bg-indigo-500/10 text-sm font-medium text-indigo-600 dark:text-indigo-200"
    >
      松开以上传到 {cwd}
    </div>
  {/if}
</div>

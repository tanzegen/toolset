<script lang="ts">
  import { onMount } from "svelte";
  import { save, open } from "@tauri-apps/plugin-dialog";
  import { cls } from "../ui";
  import { appState } from "../state.svelte";
  import Icon from "../components/Icon.svelte";
  import TermView from "../components/ssh/TermView.svelte";
  import SftpPanel from "../components/ssh/SftpPanel.svelte";
  import ConnForm from "../components/ssh/ConnForm.svelte";
  import ContextMenu from "../components/ssh/ContextMenu.svelte";
  import KeyBindings from "../components/ssh/KeyBindings.svelte";
  import { matchAction } from "../keys.svelte";
  import { resizeHandle } from "../resize";
  import {
    ssh,
    type ConnList,
    type ConnView,
    type VaultStatus,
    type ConnInput,
    type SecretInput,
  } from "../ssh";

  let vault = $state<VaultStatus>({ hasMaster: false, unlocked: false });
  let list = $state<ConnList>({ groups: [], connections: [] });
  let toast = $state("");

  // 连接列表：搜索、分组折叠、右键菜单
  type MenuItem = { label: string; icon?: string; danger?: boolean; onclick: () => void };
  let connQuery = $state("");
  let collapsedGroups = $state<string[]>(loadCollapsedGroups());
  let menu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);

  function loadCollapsedGroups(): string[] {
    if (typeof localStorage === "undefined") return [];
    try {
      return JSON.parse(localStorage.getItem("ssh-collapsed-groups") || "[]") as string[];
    } catch {
      return [];
    }
  }
  function toggleGroup(g: string) {
    const i = collapsedGroups.indexOf(g);
    if (i >= 0) collapsedGroups.splice(i, 1);
    else collapsedGroups.push(g);
    if (typeof localStorage !== "undefined")
      localStorage.setItem("ssh-collapsed-groups", JSON.stringify(collapsedGroups));
  }

  type TabKind = "term" | "sftp" | "local";
  type Tab = {
    key: string;
    connId: string;
    title: string;
    status: string;
    relogin: number;
    kind: TabKind;
    shell?: string; // 仅本地终端：powershell / cmd / ...
    searchSignal: number; // ++ 即让该标签的终端打开搜索框
  };
  let tabs = $state<Tab[]>([]);
  let activeKey = $state("");
  let showKeys = $state(false); // 快捷键设置弹窗

  // 选择导出 + 为导出重设主密码
  let exportOpen = $state(false);
  let exportIds = $state<string[]>([]);
  let exportPw = $state("");
  let exportPw2 = $state("");
  let exportErr = $state("");

  // 最近关闭的标签栈（Ctrl+Shift+T 依次重开）；仅会话内有效
  let closedTabs: { connId: string; kind: TabKind; shell?: string }[] = [];

  // 上次存续的标签（connId+kind+shell），用于重启应用后恢复。init 时同步读取，避免被持久化 effect 覆盖。
  const savedOpenTabs: { connId: string; kind: TabKind; shell?: string }[] = (() => {
    if (typeof localStorage === "undefined") return [];
    try {
      return JSON.parse(localStorage.getItem("ssh-open-tabs") || "[]");
    } catch {
      return [];
    }
  })();
  let fontSize = $state(
    (typeof localStorage !== "undefined" && Number(localStorage.getItem("ssh-font-size"))) || 13,
  );
  function setFont(n: number) {
    fontSize = Math.max(9, Math.min(22, n));
    if (typeof localStorage !== "undefined") localStorage.setItem("ssh-font-size", String(fontSize));
  }
  const activeTab = $derived(tabs.find((t) => t.key === activeKey));

  // 连接列表收起/展开（持久化）
  let connCollapsed = $state(
    typeof localStorage !== "undefined" && localStorage.getItem("ssh-conn-collapsed") === "1",
  );
  function toggleConn() {
    connCollapsed = !connCollapsed;
    if (typeof localStorage !== "undefined")
      localStorage.setItem("ssh-conn-collapsed", connCollapsed ? "1" : "0");
  }

  // 连接列表宽度（拖拽改宽，持久化）
  const CONN_MIN = 200;
  const CONN_MAX = 460;
  let connAsideEl = $state<HTMLElement>();
  let connResizing = $state(false);
  let connWidth = $state(loadConnWidth());
  function loadConnWidth(): number {
    if (typeof localStorage !== "undefined") {
      const n = Number(localStorage.getItem("ssh-conn-width"));
      if (n >= CONN_MIN && n <= CONN_MAX) return n;
    }
    return 256; // 约等于原 w-64
  }
  function setConnWidth(px: number) {
    connWidth = Math.max(CONN_MIN, Math.min(CONN_MAX, Math.round(px)));
    if (typeof localStorage !== "undefined") localStorage.setItem("ssh-conn-width", String(connWidth));
  }

  // 收起态的 vault 按钮：依状态切换 设主密码 / 解锁 / 锁定
  function vaultRailAction() {
    if (!vault.hasMaster) openPw("set");
    else if (!vault.unlocked) openPw("unlock");
    else ssh.vaultLock().then(refreshVault);
  }

  // 连接编辑表单
  let showForm = $state(false);
  let editingConn = $state<ConnView | null>(null);

  // 密码弹窗：set=设主密码 / unlock=解锁 / reset=重置 / import=输入源文件主密码
  type PwMode = "set" | "unlock" | "reset" | "import";
  let pw = $state<{ mode: PwMode; error: string; path?: string } | null>(null);
  let pwValue = $state("");
  let pending: (() => void) | null = null;

  function flash(msg: string) {
    toast = msg;
    setTimeout(() => (toast = ""), 2600);
  }

  async function refreshVault() {
    vault = await ssh.vaultStatus();
  }
  async function refreshList() {
    list = await ssh.connList();
  }

  onMount(async () => {
    await Promise.all([refreshVault(), refreshList()]);
    // 恢复上次存续的终端/SFTP/本地标签（远端连接仍存在才恢复；锁定时由 TermView 提示解锁）
    for (const s of savedOpenTabs) {
      if (s.kind === "local") {
        openLocal(s.shell || "powershell");
      } else {
        const c = list.connections.find((x) => x.id === s.connId);
        if (c) restoreTab(c, s.kind);
      }
    }
  });

  // 持久化当前打开的标签（connId+kind+shell），供下次启动恢复
  $effect(() => {
    if (typeof localStorage === "undefined") return;
    const open = tabs.map((t) => ({ connId: t.connId, kind: t.kind, shell: t.shell }));
    localStorage.setItem("ssh-open-tabs", JSON.stringify(open));
  });

  // 按分组聚合（无分组归入「未分组」），并按搜索词过滤
  const grouped = $derived.by(() => {
    const q = connQuery.trim().toLowerCase();
    const map = new Map<string, ConnView[]>();
    for (const c of list.connections) {
      if (q && !`${c.name} ${c.host} ${c.username} ${c.group}`.toLowerCase().includes(q)) continue;
      const g = c.group || "未分组";
      if (!map.has(g)) map.set(g, []);
      map.get(g)!.push(c);
    }
    return [...map.entries()];
  });
  const searching = $derived(connQuery.trim().length > 0);

  // 导出弹窗用：全部连接按分组聚合（不受搜索词影响）
  const allGroups = $derived.by(() => {
    const map = new Map<string, ConnView[]>();
    for (const c of list.connections) {
      const g = c.group || "未分组";
      if (!map.has(g)) map.set(g, []);
      map.get(g)!.push(c);
    }
    return [...map.entries()];
  });

  function openPw(mode: PwMode, path?: string) {
    pw = { mode, error: "", path };
    pwValue = "";
  }

  async function submitPw() {
    if (!pw) return;
    const { mode, path } = pw;
    const v = pwValue;
    try {
      if (mode === "set") {
        await ssh.vaultSetMaster(v);
        await refreshVault();
      } else if (mode === "unlock") {
        const ok = await ssh.vaultUnlock(v);
        if (!ok) {
          pw = { mode, error: "主密码错误", path };
          return;
        }
        await refreshVault();
      } else if (mode === "reset") {
        await ssh.vaultReset(v);
        await Promise.all([refreshVault(), refreshList()]);
        flash("已重置主密码（已清空所有已存密码）");
      } else if (mode === "import") {
        const r = await ssh.connImport(path!, v || undefined);
        await refreshList();
        flash(
          `导入 ${r.imported} 条，恢复密码 ${r.secretsRecovered} 个` +
            (r.secretsDropped ? `，丢弃 ${r.secretsDropped} 个（主密码不符或当前未解锁）` : ""),
        );
      }
      pw = null;
      const p = pending;
      pending = null;
      p?.();
    } catch (e) {
      pw = { mode, error: String(e), path };
    }
  }

  /** 需要解锁才能继续的动作：已解锁直接执行，否则弹解锁框并在成功后续跑。 */
  function requireUnlock(action: () => void) {
    if (vault.unlocked) {
      action();
      return;
    }
    pending = action;
    if (pw) return; // 已在弹解锁/设密码框，避免重复弹（恢复多标签时尤其重要）
    openPw(vault.hasMaster ? "unlock" : "set");
  }

  function openTab(c: ConnView, kind: "term" | "sftp" = "term") {
    const needSecret = c.hasPassword || c.hasKeyPem;
    if (needSecret && !vault.unlocked) {
      requireUnlock(() => openTab(c, kind));
      return;
    }
    const key = crypto.randomUUID();
    const title = (c.name || c.host) + (kind === "sftp" ? " · SFTP" : "");
    tabs.push({ key, connId: c.id, title, status: "connecting", relogin: 0, kind, searchSignal: 0 });
    activeKey = key;
  }

  // 本地终端：拉起本机 shell，无需主密码/连接（connId 留空）。
  function openLocal(shell: string) {
    const key = crypto.randomUUID();
    const title =
      shell === "cmd" ? "CMD" : shell === "pwsh" ? "pwsh" : shell === "wsl" ? "WSL" : "PowerShell";
    tabs.push({ key, connId: "", title, status: "connecting", relogin: 0, kind: "local", shell, searchSignal: 0 });
    activeKey = key;
  }
  function openLocalMenu(e: MouseEvent) {
    e.preventDefault();
    e.stopPropagation(); // 否则这次左键 click 冒泡到 window 会被 ContextMenu 立刻关掉
    menu = {
      x: e.clientX,
      y: e.clientY,
      items: [
        { label: "PowerShell", icon: "terminal", onclick: () => openLocal("powershell") },
        { label: "命令提示符 (CMD)", icon: "terminal", onclick: () => openLocal("cmd") },
      ],
    };
  }

  function closeTab(key: string) {
    const i = tabs.findIndex((t) => t.key === key);
    if (i < 0) return;
    const t = tabs[i];
    closedTabs.push({ connId: t.connId, kind: t.kind, shell: t.shell }); // 记入"最近关闭"栈
    tabs.splice(i, 1);
    if (activeKey === key) activeKey = tabs[Math.max(0, i - 1)]?.key ?? "";
  }

  // 依次重开最近关闭的标签（远端连接已删除则跳过，直到有可开或栈空）
  function reopenClosed() {
    while (closedTabs.length) {
      const s = closedTabs.pop()!;
      if (s.kind === "local") {
        openLocal(s.shell || "powershell");
        return;
      }
      const c = list.connections.find((x) => x.id === s.connId);
      if (c) {
        openTab(c, s.kind);
        return;
      }
    }
  }

  // 恢复用：直接建标签项（不经 requireUnlock 预检，连接由 TermView 发起，必要时再提示解锁）
  function restoreTab(c: ConnView, kind: "term" | "sftp") {
    const key = crypto.randomUUID();
    const title = (c.name || c.host) + (kind === "sftp" ? " · SFTP" : "");
    tabs.push({ key, connId: c.id, title, status: "connecting", relogin: 0, kind, searchSignal: 0 });
    activeKey = key;
  }

  function reloginActive() {
    const t = tabs.find((x) => x.key === activeKey);
    if (t) t.relogin++;
  }

  // 解锁后重连所有"因未解锁而卡住"的标签（一次解锁救活全部，含恢复的标签）
  function reviveLockedTabs() {
    for (const t of tabs) if (t.status === "locked") t.relogin++;
  }

  // 右键菜单：连接行 / 标签
  function openConnMenu(e: MouseEvent, c: ConnView) {
    e.preventDefault();
    e.stopPropagation();
    menu = {
      x: e.clientX,
      y: e.clientY,
      items: [
        { label: "连接（终端）", icon: "terminal", onclick: () => openTab(c) },
        { label: "打开 SFTP", icon: "folder", onclick: () => openTab(c, "sftp") },
        { label: "编辑", icon: "pencil", onclick: () => editConn(c) },
        { label: "克隆", icon: "copy", onclick: () => cloneConn(c) },
        { label: "删除", icon: "trash", danger: true, onclick: () => delConn(c) },
      ],
    };
  }
  function openTabMenu(e: MouseEvent, t: Tab) {
    e.preventDefault();
    e.stopPropagation();
    menu = {
      x: e.clientX,
      y: e.clientY,
      items: [
        { label: "克隆标签", icon: "copy", onclick: () => cloneTab(t) },
        {
          label: t.kind === "sftp" ? "刷新" : t.kind === "local" ? "重启" : "重连",
          icon: "refresh",
          onclick: () => t.relogin++,
        },
        { label: "关闭", icon: "trash", danger: true, onclick: () => closeTab(t.key) },
      ],
    };
  }
  function cloneTab(t: Tab) {
    if (t.kind === "local") {
      openLocal(t.shell || "powershell");
      return;
    }
    const c = list.connections.find((x) => x.id === t.connId);
    if (c) openTab(c, t.kind);
  }

  function newConn() {
    editingConn = null;
    showForm = true;
  }
  function editConn(c: ConnView) {
    editingConn = c;
    showForm = true;
  }

  async function saveConn(input: ConnInput, secrets?: SecretInput) {
    try {
      await ssh.connSave(input, secrets);
      await refreshList();
      showForm = false;
    } catch (e) {
      const msg = String(e);
      if (msg.includes("未解锁")) {
        showForm = false;
        requireUnlock(() => saveConn(input, secrets));
      } else {
        flash(msg);
      }
    }
  }

  async function delConn(c: ConnView) {
    if (!confirm(`删除连接「${c.name || c.host}」？`)) return;
    await ssh.connDelete(c.id);
    await refreshList();
  }

  async function cloneConn(c: ConnView) {
    await ssh.connClone(c.id);
    await refreshList();
    flash("已克隆");
  }

  // 打开「选择导出」弹窗（默认全选）
  function openExport() {
    if (list.connections.length === 0) {
      flash("还没有可导出的连接");
      return;
    }
    exportIds = list.connections.map((c) => c.id);
    exportPw = "";
    exportPw2 = "";
    exportErr = "";
    exportOpen = true;
  }
  function toggleExportId(id: string) {
    const i = exportIds.indexOf(id);
    if (i >= 0) exportIds.splice(i, 1);
    else exportIds.push(id);
  }
  function toggleExportGroup(conns: ConnView[]) {
    const ids = conns.map((c) => c.id);
    const allSel = ids.every((id) => exportIds.includes(id));
    if (allSel) exportIds = exportIds.filter((id) => !ids.includes(id));
    else for (const id of ids) if (!exportIds.includes(id)) exportIds.push(id);
  }
  const exportHasSecret = $derived(
    exportIds.some((id) => {
      const c = list.connections.find((x) => x.id === id);
      return !!c && (c.hasPassword || c.hasKeyPem || c.hasKeyPass);
    }),
  );

  async function doExportSelected() {
    exportErr = "";
    if (exportIds.length === 0) {
      exportErr = "请至少选择一个连接";
      return;
    }
    if (exportPw !== exportPw2) {
      exportErr = "两次输入的主密码不一致";
      return;
    }
    // 重设主密码需当前 vault 已解锁（才能解密原密文重加密）；先解锁再继续。
    if (exportPw && exportHasSecret && !vault.unlocked) {
      requireUnlock(() => doExportSelected());
      return;
    }
    const path = await save({
      defaultPath: "ssh-connections.json",
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path) return;
    try {
      const n = await ssh.connExportSelected(exportIds, path, exportPw || undefined);
      exportOpen = false;
      flash(
        exportPw
          ? `已导出 ${n} 条（密码已用新主密码加密）`
          : `已导出 ${n} 条（密码沿用当前主密码加密）`,
      );
    } catch (e) {
      exportErr = String(e);
    }
  }

  async function doImport() {
    const path = await open({ multiple: false, filters: [{ name: "JSON", extensions: ["json"] }] });
    if (typeof path !== "string") return;
    openPw("import", path);
  }

  // 全局快捷键（仅当 SSH 工具可见时生效）
  function onKey(e: KeyboardEvent) {
    if (appState.activeTool !== "ssh") return;
    // 在真正的表单输入框里打字时不触发；但终端（xterm 的 helper textarea）要放行，
    // 否则焦点在终端里时 Ctrl+Shift+T/R 会被误挡。
    const el = e.target as HTMLElement | null;
    const typing =
      !!el &&
      (el.tagName === "INPUT" ||
        el.isContentEditable ||
        (el.tagName === "TEXTAREA" && !el.classList.contains("xterm-helper-textarea")));
    if (typing) return;
    const id = matchAction(e);
    if (!id) return;
    e.preventDefault();
    runAction(id);
  }

  function runAction(id: string) {
    const t = tabs.find((x) => x.key === activeKey);
    switch (id) {
      case "newTab":
        reopenClosed();
        break;
      case "closeTab":
        if (t) closeTab(t.key);
        break;
      case "reconnect":
        reloginActive();
        break;
      case "nextTab":
        cycleTab(1);
        break;
      case "prevTab":
        cycleTab(-1);
        break;
      case "zoomIn":
        setFont(fontSize + 1);
        break;
      case "zoomOut":
        setFont(fontSize - 1);
        break;
      case "search":
        if (t) t.searchSignal++;
        break;
    }
  }
  function cycleTab(dir: number) {
    if (tabs.length < 2) return;
    const i = tabs.findIndex((x) => x.key === activeKey);
    if (i < 0) return;
    activeKey = tabs[(i + dir + tabs.length) % tabs.length].key;
  }

  const vaultLabel = $derived(
    !vault.hasMaster ? "未设置主密码" : vault.unlocked ? "已解锁" : "已锁定",
  );
</script>

<svelte:window onkeydown={onKey} />

<div class="flex h-full">
  <!-- 左侧：vault + 连接列表（可收起） -->
  {#if connCollapsed}
    <div class="flex w-12 shrink-0 flex-col items-center gap-1 overflow-y-auto border-r border-slate-200 py-2 dark:border-slate-800">
      <button
        class="shrink-0 rounded-md p-1.5 text-slate-400 transition hover:bg-slate-200 hover:text-slate-700 dark:hover:bg-slate-800 dark:hover:text-slate-200"
        title="展开连接列表"
        aria-label="展开连接列表"
        onclick={toggleConn}
      >
        <Icon name="chevron-left" size={18} class="rotate-180" />
      </button>

      <!-- vault：set / unlock / lock -->
      <button
        class="shrink-0 rounded-md p-1.5 transition hover:bg-slate-200 dark:hover:bg-slate-800 {!vault.hasMaster
          ? 'text-slate-400'
          : vault.unlocked
            ? 'text-emerald-500'
            : 'text-amber-500'}"
        title={!vault.hasMaster ? "设置主密码" : vault.unlocked ? "已解锁，点击锁定" : "已锁定，点击解锁"}
        aria-label="主密码"
        onclick={vaultRailAction}
      >
        <Icon name={!vault.hasMaster ? "key" : vault.unlocked ? "unlock" : "lock"} size={17} />
      </button>

      <!-- 新建连接 -->
      <button
        class="shrink-0 rounded-md p-1.5 text-slate-500 transition hover:bg-indigo-100 hover:text-indigo-700 dark:text-slate-400 dark:hover:bg-indigo-950/50"
        title="新建连接"
        aria-label="新建连接"
        onclick={newConn}
      >
        <Icon name="plus" size={18} />
      </button>

      <div class="my-0.5 w-6 shrink-0 border-t border-slate-200 dark:border-slate-800"></div>

      <!-- 服务器列表（缩小：首字母头像，点击连接） -->
      {#each list.connections as c (c.id)}
        <button
          class="flex h-8 w-8 shrink-0 items-center justify-center rounded-md text-xs font-semibold uppercase text-slate-600 transition hover:bg-indigo-100 hover:text-indigo-700 dark:text-slate-300 dark:hover:bg-indigo-950/50"
          title={`${c.name || c.host}  ·  ${c.username}@${c.host}:${c.port}`}
          aria-label={c.name || c.host}
          onclick={() => openTab(c)}
          ondblclick={() => openTab(c, "sftp")}
        >
          {(c.name || c.host).slice(0, 1)}
        </button>
      {/each}
    </div>
  {:else}
  <aside
    bind:this={connAsideEl}
    class="relative flex shrink-0 flex-col border-r border-slate-200 dark:border-slate-800"
    style="width:{connWidth}px"
  >
    <div class="border-b border-slate-200 p-3 dark:border-slate-800">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-1.5">
          <button
            class="rounded p-0.5 text-slate-400 transition hover:bg-slate-200 hover:text-slate-700 dark:hover:bg-slate-800 dark:hover:text-slate-200"
            title="收起连接列表"
            aria-label="收起连接列表"
            onclick={toggleConn}
          >
            <Icon name="chevron-left" size={16} />
          </button>
          <span class="text-sm font-semibold text-slate-700 dark:text-slate-200">SSH 连接</span>
        </div>
        <span
          class="rounded px-1.5 py-0.5 text-[10px] {vault.unlocked
            ? 'bg-emerald-50 text-emerald-600 dark:bg-emerald-950/40 dark:text-emerald-300'
            : 'bg-amber-50 text-amber-600 dark:bg-amber-950/40 dark:text-amber-300'}"
        >
          {vaultLabel}
        </span>
      </div>
      <div class="mt-2 flex flex-wrap gap-1.5">
        {#if !vault.hasMaster}
          <button class="{cls.btn} px-2 py-1 text-xs" onclick={() => openPw("set")}>设置主密码</button>
        {:else if !vault.unlocked}
          <button class="{cls.btn} px-2 py-1 text-xs" onclick={() => openPw("unlock")}>解锁</button>
        {:else}
          <button class="{cls.btn} px-2 py-1 text-xs" onclick={async () => { await ssh.vaultLock(); refreshVault(); }}>锁定</button>
          <button class="{cls.btn} px-2 py-1 text-xs" onclick={() => openPw("reset")}>重置</button>
        {/if}
      </div>
    </div>

    <div class="flex items-center gap-1.5 px-3 py-2">
      <button class="{cls.btnPrimary} flex-1 px-2 py-1.5 text-xs" onclick={newConn}>＋ 新建</button>
      <button class="{cls.btn} px-2 py-1.5 text-xs" onclick={doImport} title="导入">导入</button>
      <button class="{cls.btn} px-2 py-1.5 text-xs" onclick={openExport} title="选择导出">导出</button>
    </div>

    <!-- 搜索 -->
    <div class="px-3 pb-1">
      <div class="relative">
        <span class="pointer-events-none absolute left-2 top-1.5 text-slate-400"><Icon name="search" size={13} /></span>
        <input
          bind:value={connQuery}
          placeholder="搜索连接…"
          class="w-full rounded-md border border-slate-200 bg-white py-1 pl-7 pr-2 text-xs outline-none transition focus:border-indigo-500 focus:ring-2 focus:ring-indigo-500/30 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-100"
        />
      </div>
    </div>

    <div class="flex-1 overflow-y-auto px-2 pb-3">
      {#each grouped as [g, conns] (g)}
        {@const expanded = searching || !collapsedGroups.includes(g)}
        <div class="mt-2">
          <button
            class="flex w-full items-center gap-1 rounded px-1.5 py-1 text-[11px] font-medium uppercase tracking-wide text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800/60"
            onclick={() => toggleGroup(g)}
            title="折叠 / 展开分组"
          >
            <Icon name="chevron-left" size={12} class="shrink-0 transition {expanded ? '-rotate-90' : 'rotate-180'}" />
            <span class="truncate">{g}</span>
            <span class="ml-auto text-slate-400">{conns.length}</span>
          </button>
          {#if expanded}
            {#each conns as c (c.id)}
              <div
                class="group flex items-center gap-1 rounded-lg px-2 py-1.5 hover:bg-slate-100 dark:hover:bg-slate-800/60"
                oncontextmenu={(e) => openConnMenu(e, c)}
              >
                <button class="min-w-0 flex-1 text-left" onclick={() => openTab(c)} title="连接（右键更多操作）">
                  <div class="truncate text-sm text-slate-700 dark:text-slate-200">{c.name || c.host}</div>
                  <div class="truncate font-mono text-[11px] text-slate-400">{c.username}@{c.host}:{c.port}</div>
                </button>
                <div class="flex shrink-0 items-center gap-0.5 opacity-0 transition group-hover:opacity-100">
                  <button class="rounded p-1 hover:bg-slate-200 dark:hover:bg-slate-700" title="文件 (SFTP)" onclick={() => openTab(c, "sftp")}><Icon name="folder" size={13} /></button>
                  <button class="rounded p-1 hover:bg-slate-200 dark:hover:bg-slate-700" title="编辑" onclick={() => editConn(c)}><Icon name="pencil" size={13} /></button>
                  <button class="rounded p-1 hover:bg-slate-200 dark:hover:bg-slate-700" title="克隆" onclick={() => cloneConn(c)}><Icon name="copy" size={13} /></button>
                  <button class="rounded p-1 hover:bg-slate-200 dark:hover:bg-slate-700" title="删除" onclick={() => delConn(c)}><Icon name="trash" size={13} /></button>
                </div>
              </div>
            {/each}
          {/if}
        </div>
      {/each}
      {#if list.connections.length === 0}
        <p class="px-3 py-6 text-center text-xs text-slate-400">还没有连接，点「＋ 新建」</p>
      {:else if grouped.length === 0}
        <p class="px-3 py-6 text-center text-xs text-slate-400">无匹配连接</p>
      {/if}
    </div>
    <!-- 右缘拖拽改宽 -->
    <div
      role="separator"
      aria-orientation="vertical"
      title="拖动调整宽度"
      class="absolute right-0 top-0 z-10 h-full w-1.5 cursor-col-resize transition hover:bg-indigo-400/40 {connResizing
        ? 'bg-indigo-400/50'
        : ''}"
      use:resizeHandle={{
        onstart: () => (connResizing = true),
        onend: () => (connResizing = false),
        onmove: (clientX) => {
          if (connAsideEl) setConnWidth(clientX - connAsideEl.getBoundingClientRect().left);
        },
      }}
    ></div>
  </aside>
  {/if}

  <!-- 右侧：标签 + 终端 -->
  <section class="flex min-w-0 flex-1 flex-col">
    <div class="flex items-center gap-1 border-b border-slate-200 px-2 py-1.5 dark:border-slate-800">
      <div class="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto">
        {#each tabs as t (t.key)}
          <div
            class="flex shrink-0 items-center gap-1.5 rounded-t-lg border-b-2 px-3 py-1 text-xs {activeKey === t.key
              ? 'border-indigo-500 bg-slate-100 text-slate-800 dark:bg-slate-800 dark:text-slate-100'
              : 'border-transparent text-slate-500 hover:bg-slate-50 dark:hover:bg-slate-800/50'}"
            oncontextmenu={(e) => openTabMenu(e, t)}
          >
            <button onclick={() => (activeKey = t.key)} class="flex items-center gap-1.5">
              <span
                class="h-1.5 w-1.5 rounded-full {t.status === 'connected'
                  ? 'bg-emerald-500'
                  : t.status === 'closed' || t.status === 'error' || t.status === 'exited'
                    ? 'bg-red-500'
                    : 'bg-amber-400'}"
              ></span>
              <span class="max-w-32 truncate">{t.title}</span>
            </button>
            <button class="rounded hover:text-red-500" title="关闭" onclick={() => closeTab(t.key)}>✕</button>
          </div>
        {/each}
      </div>
      {#if activeTab?.kind === "term" || activeTab?.kind === "local"}
        <button class="{cls.btn} px-2 py-1 text-xs" onclick={reloginActive} title={activeTab?.kind === "local" ? "重启" : "重连"}>{activeTab?.kind === "local" ? "重启" : "重连"}</button>
        <button class="{cls.btn} px-2 py-1 text-xs" onclick={() => setFont(fontSize - 1)} title="减小字号">A-</button>
        <button class="{cls.btn} px-2 py-1 text-xs" onclick={() => setFont(fontSize + 1)} title="增大字号">A+</button>
      {/if}
      <button
        class="{cls.btn} px-2 py-1 text-xs"
        onclick={() => openLocal("powershell")}
        oncontextmenu={openLocalMenu}
        title="新建本地终端（PowerShell · 右键选其它）"
        aria-label="新建本地终端"
      >
        <Icon name="monitor" size={14} />
      </button>
      <button class="{cls.btn} px-2 py-1 text-xs" onclick={() => (showKeys = true)} title="快捷键设置" aria-label="快捷键设置">
        <Icon name="keyboard" size={14} />
      </button>
    </div>

    <div class="relative min-h-0 flex-1 bg-[#0b1020] p-2">
      {#each tabs as t (t.key)}
        <div class="absolute inset-2" class:hidden={activeKey !== t.key}>
          {#if t.kind === "sftp"}
            {#key t.relogin}
              <SftpPanel connId={t.connId} onlocked={() => requireUnlock(() => t.relogin++)} />
            {/key}
          {:else if t.kind === "local"}
            <TermView
              connId=""
              mode="local"
              shell={t.shell}
              {fontSize}
              relogin={t.relogin}
              active={activeKey === t.key}
              searchSignal={t.searchSignal}
              onstatus={(s) => (t.status = s)}
              onexit={() => closeTab(t.key)}
            />
          {:else}
            <TermView
              connId={t.connId}
              {fontSize}
              relogin={t.relogin}
              active={activeKey === t.key}
              searchSignal={t.searchSignal}
              onstatus={(s) => (t.status = s)}
              onlocked={() => requireUnlock(reviveLockedTabs)}
              onexit={() => closeTab(t.key)}
            />
          {/if}
        </div>
      {/each}
      {#if tabs.length === 0}
        <div class="flex h-full items-center justify-center px-6 text-center text-sm text-slate-500">
          从左侧选择一个连接，或点上方 ▥ 新建本地终端（Ctrl+Shift+T 重开最近关闭 · Ctrl+Shift+R 重连）
        </div>
      {/if}
    </div>
  </section>
</div>

{#if menu}
  <ContextMenu x={menu.x} y={menu.y} items={menu.items} onclose={() => (menu = null)} />
{/if}

{#if showKeys}
  <KeyBindings onclose={() => (showKeys = false)} />
{/if}

{#if showForm}
  <ConnForm conn={editingConn} groups={list.groups} onsave={saveConn} oncancel={() => (showForm = false)} />
{/if}

{#if exportOpen}
  <div class="fixed inset-0 z-40 flex items-center justify-center bg-black/40 p-4" role="presentation" onclick={() => (exportOpen = false)}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div
      class="flex max-h-[85vh] w-full max-w-lg flex-col rounded-xl border border-slate-200 bg-white p-5 shadow-xl dark:border-slate-700 dark:bg-slate-900"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="mb-1 flex items-center justify-between">
        <h2 class="text-base font-semibold text-slate-800 dark:text-slate-100">选择导出</h2>
        <div class="flex gap-1 text-xs">
          <button class="{cls.btn} px-2 py-1" onclick={() => (exportIds = list.connections.map((c) => c.id))}>全选</button>
          <button class="{cls.btn} px-2 py-1" onclick={() => (exportIds = [])}>全不选</button>
        </div>
      </div>
      <p class="mb-2 text-xs text-slate-400">
        勾选要导出的连接。可为这份导出<strong>重设主密码</strong>——留空则沿用当前主密码（导入端需用对应主密码恢复）。
      </p>

      <div class="min-h-0 flex-1 overflow-y-auto rounded-lg border border-slate-200 p-1 dark:border-slate-700">
        {#each allGroups as [g, conns] (g)}
          {@const sel = conns.filter((c) => exportIds.includes(c.id)).length}
          <div class="mt-1 first:mt-0">
            <button
              class="flex w-full items-center gap-1 rounded px-1.5 py-1 text-[11px] font-medium uppercase tracking-wide text-slate-400 hover:bg-slate-100 dark:hover:bg-slate-800/60"
              onclick={() => toggleExportGroup(conns)}
              title="全选/取消该分组"
            >
              <span class="truncate">{g}</span>
              <span class="ml-auto">{sel}/{conns.length}</span>
            </button>
            {#each conns as c (c.id)}
              <label class="flex cursor-pointer items-center gap-2 rounded-lg px-2 py-1.5 hover:bg-slate-50 dark:hover:bg-slate-800/50">
                <input
                  type="checkbox"
                  class="accent-indigo-600"
                  checked={exportIds.includes(c.id)}
                  onchange={() => toggleExportId(c.id)}
                />
                <span class="truncate text-sm text-slate-700 dark:text-slate-200">{c.name || c.host}</span>
                <span class="ml-auto flex items-center gap-1 truncate font-mono text-[11px] text-slate-400">
                  {c.username}@{c.host}:{c.port}
                  {#if c.hasPassword || c.hasKeyPem || c.hasKeyPass}<Icon name="lock" size={11} />{/if}
                </span>
              </label>
            {/each}
          </div>
        {/each}
      </div>

      <div class="mt-3 space-y-2">
        <input type="password" bind:value={exportPw} placeholder="新主密码（留空＝沿用当前主密码）" class="{cls.field} font-mono" />
        {#if exportPw}
          <input type="password" bind:value={exportPw2} placeholder="确认新主密码" class="{cls.field} font-mono" />
        {/if}
        {#if exportPw && exportHasSecret && !vault.unlocked}
          <p class="text-xs text-amber-500">重设主密码需先解锁当前主密码（点「导出」时会提示解锁）。</p>
        {/if}
      </div>

      {#if exportErr}<p class="mt-2 text-sm text-red-600 dark:text-red-400">{exportErr}</p>{/if}
      <div class="mt-4 flex items-center justify-between">
        <span class="text-xs text-slate-400">已选 {exportIds.length} / {list.connections.length}</span>
        <div class="flex gap-2">
          <button class={cls.btn} onclick={() => (exportOpen = false)}>取消</button>
          <button class={cls.btnPrimary} onclick={doExportSelected}>导出…</button>
        </div>
      </div>
    </div>
  </div>
{/if}

{#if pw}
  <div class="fixed inset-0 z-40 flex items-center justify-center bg-black/40 p-4" role="presentation" onclick={() => (pw = null)}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="w-full max-w-sm rounded-xl border border-slate-200 bg-white p-5 shadow-xl dark:border-slate-700 dark:bg-slate-900" onclick={(e) => e.stopPropagation()}>
      <h2 class="mb-1 text-base font-semibold text-slate-800 dark:text-slate-100">
        {pw.mode === "set" ? "设置主密码" : pw.mode === "unlock" ? "解锁主密码" : pw.mode === "reset" ? "重置主密码" : "导入：源文件主密码"}
      </h2>
      <p class="mb-3 text-xs text-slate-400">
        {#if pw.mode === "set"}用于加密所有连接的密码 / 私钥。无法找回——忘记将无法解密已存密码。
        {:else if pw.mode === "reset"}将清空所有已保存的密码 / 私钥（连接本身保留），并设为新主密码。
        {:else if pw.mode === "import"}留空则只导入连接、不含密码；要恢复密码需填源文件主密码且当前已解锁。
        {:else}输入主密码以解锁本次会话。{/if}
      </p>
      <input
        type="password"
        bind:value={pwValue}
        autofocus
        onkeydown={(e) => e.key === "Enter" && submitPw()}
        placeholder={pw.mode === "import" ? "源文件主密码（可留空）" : "主密码"}
        class="{cls.field} font-mono"
      />
      {#if pw.error}<p class="mt-2 text-sm text-red-600 dark:text-red-400">{pw.error}</p>{/if}
      <div class="mt-4 flex justify-end gap-2">
        <button class={cls.btn} onclick={() => (pw = null)}>取消</button>
        <button class={cls.btnPrimary} onclick={submitPw}>确定</button>
      </div>
    </div>
  </div>
{/if}

{#if toast}
  <div class="fixed bottom-4 left-1/2 z-50 -translate-x-1/2 rounded-lg bg-slate-800 px-4 py-2 text-sm text-white shadow-lg dark:bg-slate-700">
    {toast}
  </div>
{/if}

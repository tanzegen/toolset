<script lang="ts">
  import { onMount } from "svelte";
  import { save, open } from "@tauri-apps/plugin-dialog";
  import { cls } from "../ui";
  import { appState } from "../state.svelte";
  import Icon from "../components/Icon.svelte";
  import TermView from "../components/ssh/TermView.svelte";
  import SftpPanel from "../components/ssh/SftpPanel.svelte";
  import ConnForm from "../components/ssh/ConnForm.svelte";
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

  type Tab = {
    key: string;
    connId: string;
    title: string;
    status: string;
    relogin: number;
    kind: "term" | "sftp";
  };
  let tabs = $state<Tab[]>([]);
  let activeKey = $state("");
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
  });

  // 按分组聚合（无分组归入「未分组」）
  const grouped = $derived.by(() => {
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
    } else if (!vault.hasMaster) {
      pending = action;
      openPw("set");
    } else {
      pending = action;
      openPw("unlock");
    }
  }

  function openTab(c: ConnView, kind: "term" | "sftp" = "term") {
    const needSecret = c.hasPassword || c.hasKeyPem;
    if (needSecret && !vault.unlocked) {
      requireUnlock(() => openTab(c, kind));
      return;
    }
    const key = crypto.randomUUID();
    const title = (c.name || c.host) + (kind === "sftp" ? " · SFTP" : "");
    tabs.push({ key, connId: c.id, title, status: "connecting", relogin: 0, kind });
    activeKey = key;
  }

  function closeTab(key: string) {
    const i = tabs.findIndex((t) => t.key === key);
    if (i < 0) return;
    tabs.splice(i, 1);
    if (activeKey === key) activeKey = tabs[Math.max(0, i - 1)]?.key ?? "";
  }

  function reloginActive() {
    const t = tabs.find((x) => x.key === activeKey);
    if (t) t.relogin++;
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

  async function doExport() {
    const path = await save({
      defaultPath: "ssh-connections.json",
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path) return;
    await ssh.connExport(path);
    flash("已导出（含加密后的密码字段）");
  }

  async function doImport() {
    const path = await open({ multiple: false, filters: [{ name: "JSON", extensions: ["json"] }] });
    if (typeof path !== "string") return;
    openPw("import", path);
  }

  // 全局快捷键（仅当 SSH 工具可见时生效）
  function onKey(e: KeyboardEvent) {
    if (appState.activeTool !== "ssh") return;
    // 正在输入框/文本域里打字时，不触发全局快捷键
    const el = e.target as HTMLElement | null;
    if (el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable)) return;
    if (e.ctrlKey && e.shiftKey && (e.key === "T" || e.key === "t")) {
      e.preventDefault();
      const t = tabs.find((x) => x.key === activeKey);
      const c = t && list.connections.find((x) => x.id === t.connId);
      if (c) openTab(c, t?.kind ?? "term");
    } else if (e.ctrlKey && e.shiftKey && (e.key === "R" || e.key === "r")) {
      e.preventDefault();
      reloginActive();
    }
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
  <aside class="flex w-64 shrink-0 flex-col border-r border-slate-200 dark:border-slate-800">
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
      <button class="{cls.btn} px-2 py-1.5 text-xs" onclick={doExport} title="导出">导出</button>
    </div>

    <div class="flex-1 overflow-y-auto px-2 pb-3">
      {#each grouped as [g, conns] (g)}
        <div class="mt-2">
          <div class="px-2 py-1 text-[11px] font-medium uppercase tracking-wide text-slate-400">{g}</div>
          {#each conns as c (c.id)}
            <div class="group flex items-center gap-1 rounded-lg px-2 py-1.5 hover:bg-slate-100 dark:hover:bg-slate-800/60">
              <button class="min-w-0 flex-1 text-left" onclick={() => openTab(c)} title="连接">
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
        </div>
      {/each}
      {#if list.connections.length === 0}
        <p class="px-3 py-6 text-center text-xs text-slate-400">还没有连接，点「＋ 新建」</p>
      {/if}
    </div>
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
      {#if activeTab?.kind === "term"}
        <button class="{cls.btn} px-2 py-1 text-xs" onclick={reloginActive} title="重新登录 (Ctrl+Shift+R)">重连</button>
        <button class="{cls.btn} px-2 py-1 text-xs" onclick={() => setFont(fontSize - 1)} title="减小字号">A-</button>
        <button class="{cls.btn} px-2 py-1 text-xs" onclick={() => setFont(fontSize + 1)} title="增大字号">A+</button>
      {/if}
    </div>

    <div class="relative min-h-0 flex-1 bg-[#0b1020] p-2">
      {#each tabs as t (t.key)}
        <div class="absolute inset-2" class:hidden={activeKey !== t.key}>
          {#if t.kind === "sftp"}
            {#key t.relogin}
              <SftpPanel connId={t.connId} onlocked={() => requireUnlock(() => t.relogin++)} />
            {/key}
          {:else}
            <TermView
              connId={t.connId}
              {fontSize}
              relogin={t.relogin}
              active={activeKey === t.key}
              onstatus={(s) => (t.status = s)}
              onlocked={() => requireUnlock(() => t.relogin++)}
            />
          {/if}
        </div>
      {/each}
      {#if tabs.length === 0}
        <div class="flex h-full items-center justify-center text-sm text-slate-500">
          从左侧选择一个连接开始（Ctrl+Shift+T 新标签 · Ctrl+Shift+R 重连）
        </div>
      {/if}
    </div>
  </section>
</div>

{#if showForm}
  <ConnForm conn={editingConn} groups={list.groups} onsave={saveConn} oncancel={() => (showForm = false)} />
{/if}

{#if pw}
  <div class="fixed inset-0 z-40 flex items-center justify-center bg-black/40 p-4" role="presentation" onclick={() => (pw = null)}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="{cls.card} w-full max-w-sm p-5 shadow-xl" onclick={(e) => e.stopPropagation()}>
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

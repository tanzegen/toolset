<script lang="ts">
  import { cls } from "../ui";
  import { persist } from "../persist.svelte";
  import Icon from "../components/Icon.svelte";
  import CopyButton from "../components/CopyButton.svelte";
  import Stepper from "../components/Stepper.svelte";
  import { net, type HttpResp } from "../net";

  type KV = { key: string; value: string; on: boolean };
  type BodyType = "none" | "json" | "raw" | "form";
  type Auth = {
    type: "none" | "bearer" | "basic" | "header";
    token: string;
    user: string;
    pass: string;
    headerKey: string;
    headerVal: string;
  };
  type Snapshot = {
    method: string;
    url: string;
    params: KV[];
    headers: KV[];
    bodyType: BodyType;
    body: string;
    form: KV[];
    auth: Auth;
    followRedirects: boolean;
    timeoutMs: number;
    skipTlsVerify: boolean;
  };
  type HistItem = Snapshot & { status: number; timeMs: number; at: number };
  type SavedItem = Snapshot & { id: string; name: string };

  const blank = (): KV => ({ key: "", value: "", on: true });
  const dc = <T,>(x: T): T => JSON.parse(JSON.stringify(x));

  const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

  let method = $state("GET");
  let url = $state("");
  let params = $state<KV[]>([blank()]);
  let headers = $state<KV[]>([blank()]);
  let bodyType = $state<BodyType>("none");
  let body = $state("");
  let form = $state<KV[]>([blank()]);
  let auth = $state<Auth>({ type: "none", token: "", user: "", pass: "", headerKey: "", headerVal: "" });
  let followRedirects = $state(true);
  let timeoutMs = $state(30000);
  let skipTlsVerify = $state(false);

  let vars = $state<KV[]>([blank()]);
  let history = $state<HistItem[]>([]);
  let saved = $state<SavedItem[]>([]);

  let section = $state<"params" | "headers" | "body" | "auth" | "opts">("params");
  let respTab = $state<"body" | "headers">("body");
  let pretty = $state(true);
  let bodySearch = $state("");

  let resp = $state<HttpResp | null>(null);
  let loading = $state(false);
  let err = $state("");

  // 弹窗
  let showVars = $state(false);
  let showHist = $state(false);
  let showSaved = $state(false);
  let showCurl = $state(false);
  let curlText = $state("");

  persist("http-client", {
    save: () => ({
      method, url, params, headers, bodyType, body, form, auth,
      followRedirects, timeoutMs, skipTlsVerify, vars, history, saved, section, respTab, pretty,
    }),
    load: (s) => {
      method = s.method ?? method;
      url = s.url ?? url;
      params = s.params ?? params;
      headers = s.headers ?? headers;
      bodyType = s.bodyType ?? bodyType;
      body = s.body ?? body;
      form = s.form ?? form;
      auth = s.auth ?? auth;
      followRedirects = s.followRedirects ?? followRedirects;
      timeoutMs = s.timeoutMs ?? timeoutMs;
      skipTlsVerify = s.skipTlsVerify ?? skipTlsVerify;
      vars = s.vars ?? vars;
      history = s.history ?? history;
      saved = s.saved ?? saved;
      section = s.section ?? section;
      respTab = s.respTab ?? respTab;
      pretty = s.pretty ?? pretty;
    },
  });

  function grow(rows: KV[]) {
    const last = rows[rows.length - 1];
    if (!last || last.key || last.value) rows.push(blank());
  }
  function removeRow(rows: KV[], i: number) {
    rows.splice(i, 1);
    if (rows.length === 0) rows.push(blank());
  }

  const onCount = (rows: KV[]) => rows.filter((r) => r.on && r.key.trim()).length;
  const enabled = (rows: KV[]) =>
    rows.filter((r) => r.on && r.key.trim()).map((r) => ({ key: r.key, value: r.value }));

  function authHeaders(): { key: string; value: string }[] {
    if (auth.type === "bearer" && auth.token) return [{ key: "Authorization", value: `Bearer ${auth.token}` }];
    if (auth.type === "basic") return [{ key: "Authorization", value: `Basic ${btoa(`${auth.user}:${auth.pass}`)}` }];
    if (auth.type === "header" && auth.headerKey) return [{ key: auth.headerKey, value: auth.headerVal }];
    return [];
  }

  function snapshot(): Snapshot {
    return dc({ method, url, params, headers, bodyType, body, form, auth, followRedirects, timeoutMs, skipTlsVerify });
  }
  function loadSnapshot(s: Snapshot) {
    const c = dc(s);
    method = c.method;
    url = c.url;
    params = c.params.length ? c.params : [blank()];
    headers = c.headers.length ? c.headers : [blank()];
    bodyType = c.bodyType;
    body = c.body;
    form = c.form.length ? c.form : [blank()];
    auth = c.auth;
    followRedirects = c.followRedirects;
    timeoutMs = c.timeoutMs;
    skipTlsVerify = c.skipTlsVerify;
  }

  function builtUrl(): string {
    const ps = enabled(params);
    if (!ps.length) return url;
    const qs = ps.map((p) => `${encodeURIComponent(p.key)}=${encodeURIComponent(p.value)}`).join("&");
    return url + (url.includes("?") ? "&" : "?") + qs;
  }

  async function send() {
    if (!url.trim()) return;
    loading = true;
    err = "";
    try {
      const r = await net.httpSend({
        method,
        url,
        params: enabled(params),
        headers: [...enabled(headers), ...authHeaders()],
        bodyType,
        body,
        form: enabled(form),
        vars: vars.filter((v) => v.on && v.key.trim()).map((v) => ({ key: v.key, value: v.value })),
        followRedirects,
        timeoutMs,
        skipTlsVerify,
      });
      resp = r;
      respTab = "body";
      history.unshift({ ...snapshot(), status: r.status, timeMs: r.timeMs, at: Date.now() });
      if (history.length > 30) history.length = 30;
    } catch (e) {
      err = String(e);
      resp = null;
    } finally {
      loading = false;
    }
  }

  function saveCurrent() {
    const name = prompt("保存为（名称）：", url);
    if (!name) return;
    saved.unshift({ id: crypto.randomUUID(), name, ...snapshot() });
  }

  async function importCurl() {
    err = "";
    try {
      const p = await net.curlParse(curlText);
      method = p.method;
      url = p.url;
      headers = [...p.headers.map((h) => ({ key: h.key, value: h.value, on: true })), blank()];
      if (p.body) {
        try {
          JSON.parse(p.body);
          bodyType = "json";
        } catch {
          bodyType = "raw";
        }
        body = p.body;
      }
      if (p.user) {
        const idx = p.user.indexOf(":");
        auth = { ...auth, type: "basic", user: idx >= 0 ? p.user.slice(0, idx) : p.user, pass: idx >= 0 ? p.user.slice(idx + 1) : "" };
      }
      showCurl = false;
      curlText = "";
    } catch (e) {
      err = String(e);
    }
  }

  function exportCurl(): string {
    const q = (s: string) => `'${s.replace(/'/g, "'\\''")}'`;
    const parts = [`curl -X ${method} ${q(builtUrl())}`];
    for (const h of [...enabled(headers), ...authHeaders()]) parts.push(`-H ${q(`${h.key}: ${h.value}`)}`);
    if ((bodyType === "json" || bodyType === "raw") && body) parts.push(`--data-raw ${q(body)}`);
    if (bodyType === "form") {
      const f = enabled(form).map((k) => `${k.key}=${k.value}`).join("&");
      if (f) parts.push(`--data ${q(f)}`);
    }
    if (skipTlsVerify) parts.push("-k");
    return parts.join(" \\\n  ");
  }

  const prettyBody = $derived.by(() => {
    if (!resp) return "";
    if (!pretty) return resp.body;
    try {
      return JSON.stringify(JSON.parse(resp.body), null, 2);
    } catch {
      return resp.body;
    }
  });

  const bodySegments = $derived.by(() => {
    const text = prettyBody;
    const q = bodySearch.trim();
    if (!q || text.length > 200000) return [{ t: text, m: false }];
    const segs: { t: string; m: boolean }[] = [];
    const low = text.toLowerCase();
    const lq = q.toLowerCase();
    let i = 0;
    for (;;) {
      const idx = low.indexOf(lq, i);
      if (idx < 0) {
        segs.push({ t: text.slice(i), m: false });
        break;
      }
      if (idx > i) segs.push({ t: text.slice(i, idx), m: false });
      segs.push({ t: text.slice(idx, idx + q.length), m: true });
      i = idx + q.length;
    }
    return segs;
  });

  function statusColor(s: number): string {
    if (s >= 200 && s < 300) return "text-emerald-600 dark:text-emerald-400";
    if (s >= 300 && s < 400) return "text-sky-600 dark:text-sky-400";
    if (s >= 400 && s < 500) return "text-amber-600 dark:text-amber-400";
    return "text-red-600 dark:text-red-400";
  }
  function humanSize(n: number): string {
    if (n < 1024) return `${n} B`;
    if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
    return `${(n / 1024 / 1024).toFixed(2)} MB`;
  }
  const fmtTime = (t: number) => new Date(t).toLocaleString();
</script>

<div class="flex h-full flex-col overflow-hidden">
  <!-- 顶部：主行（方法 + 地址 + 发送，始终一行）+ 次行（辅助操作） -->
  <div class="border-b border-slate-200 dark:border-slate-800">
    <div class="flex items-center gap-1.5 p-2 pb-1.5">
      <select bind:value={method} style="width:6rem" class="{cls.field} shrink-0 px-2 py-1.5 font-mono text-sm font-semibold">
        {#each METHODS as m (m)}<option value={m}>{m}</option>{/each}
      </select>
      <input
        bind:value={url}
        onkeydown={(e) => e.key === "Enter" && send()}
        placeholder="https://api.example.com/path  （支持 {'{{'}变量{'}}'}）"
        class="{cls.field} min-w-0 flex-1 py-1.5 font-mono text-sm"
      />
      <button class="{cls.btnPrimary} shrink-0 px-4 py-1.5" onclick={send} disabled={loading}>
        <Icon name="send" size={15} />{loading ? "发送中…" : "发送"}
      </button>
    </div>
    <div class="flex flex-wrap items-center gap-1.5 px-2 pb-2">
      <button class="{cls.btn} gap-1 px-2 py-1 text-xs" onclick={() => (showVars = true)} title="环境变量"><Icon name="braces" size={13} />变量</button>
      <button class="{cls.btn} gap-1 px-2 py-1 text-xs" onclick={() => (showCurl = true)} title="导入/导出 curl">curl</button>
      <button class="{cls.btn} gap-1 px-2 py-1 text-xs" onclick={saveCurrent} title="收藏当前请求"><Icon name="pin" size={13} />收藏</button>
      <button class="{cls.btn} gap-1 px-2 py-1 text-xs" onclick={() => (showSaved = true)} title="收藏夹"><Icon name="folder" size={13} />收藏夹</button>
      <button class="{cls.btn} gap-1 px-2 py-1 text-xs" onclick={() => (showHist = true)} title="历史"><Icon name="clock" size={13} />历史</button>
    </div>
  </div>

  <!-- 请求区 -->
  <div class="flex shrink-0 flex-col border-b border-slate-200 dark:border-slate-800" style="max-height:42%">
    <div class="flex items-center gap-1 px-2 pt-2 text-xs">
      {#each [["params", `Params${onCount(params) ? " " + onCount(params) : ""}`], ["headers", `Headers${onCount(headers) ? " " + onCount(headers) : ""}`], ["body", "Body"], ["auth", "Auth"], ["opts", "选项"]] as [id, label] (id)}
        <button
          class="rounded-t-md px-3 py-1 {section === id
            ? 'bg-slate-100 font-medium text-indigo-600 dark:bg-slate-800 dark:text-indigo-300'
            : 'text-slate-500 hover:bg-slate-50 dark:hover:bg-slate-800/50'}"
          onclick={() => (section = id as typeof section)}
        >{label}</button>
      {/each}
    </div>
    <div class="min-h-0 flex-1 overflow-y-auto p-3">
      {#if section === "params"}
        {@render kvTable(params)}
      {:else if section === "headers"}
        {@render kvTable(headers)}
      {:else if section === "body"}
        <div class="mb-2 flex gap-1 text-xs">
          {#each ["none", "json", "raw", "form"] as t (t)}
            <button
              class="rounded px-2 py-1 {bodyType === t
                ? 'bg-indigo-600 text-white'
                : 'border border-slate-200 text-slate-600 dark:border-slate-700 dark:text-slate-300'}"
              onclick={() => (bodyType = t as BodyType)}
            >{t}</button>
          {/each}
        </div>
        {#if bodyType === "json" || bodyType === "raw"}
          <textarea bind:value={body} rows="6" placeholder={bodyType === "json" ? '{ "key": "value" }' : "原始请求体"} class="{cls.field} w-full font-mono text-xs"></textarea>
        {:else if bodyType === "form"}
          {@render kvTable(form)}
        {:else}
          <p class="text-xs text-slate-400">该请求不带 Body。</p>
        {/if}
      {:else if section === "auth"}
        <div class="space-y-2">
          <div class="flex gap-1 text-xs">
            {#each [["none", "无"], ["bearer", "Bearer"], ["basic", "Basic"], ["header", "自定义头"]] as [t, label] (t)}
              <button
                class="rounded px-2 py-1 {auth.type === t
                  ? 'bg-indigo-600 text-white'
                  : 'border border-slate-200 text-slate-600 dark:border-slate-700 dark:text-slate-300'}"
                onclick={() => (auth.type = t as Auth["type"])}
              >{label}</button>
            {/each}
          </div>
          {#if auth.type === "bearer"}
            <input bind:value={auth.token} placeholder="Token" class="{cls.field} font-mono text-xs" />
          {:else if auth.type === "basic"}
            <input bind:value={auth.user} placeholder="用户名" class="{cls.field} text-xs" />
            <input bind:value={auth.pass} type="password" placeholder="密码" class="{cls.field} text-xs" />
          {:else if auth.type === "header"}
            <input bind:value={auth.headerKey} placeholder="头名（如 X-API-Key）" class="{cls.field} font-mono text-xs" />
            <input bind:value={auth.headerVal} placeholder="头值" class="{cls.field} font-mono text-xs" />
          {/if}
        </div>
      {:else if section === "opts"}
        <div class="space-y-2 text-sm">
          <label class="flex items-center gap-2"><input type="checkbox" class="accent-indigo-600" bind:checked={followRedirects} /> 跟随重定向</label>
          <label class="flex items-center gap-2">超时(ms)<Stepper bind:value={timeoutMs} min={0} step={1000} width="w-16" /></label>
          <label class="flex items-center gap-2 text-red-600 dark:text-red-400"><input type="checkbox" class="accent-red-600" bind:checked={skipTlsVerify} /> ⚠ 跳过 TLS 证书校验（仅内网自签用）</label>
        </div>
      {/if}
    </div>
  </div>

  <!-- 响应区 -->
  <div class="flex min-h-0 flex-1 flex-col">
    {#if err}
      <p class="m-3 rounded-lg bg-red-50 px-3 py-2 text-sm text-red-600 dark:bg-red-950/40 dark:text-red-400">{err}</p>
    {/if}
    {#if resp}
      <div class="flex flex-wrap items-center gap-3 border-b border-slate-200 px-3 py-2 text-xs dark:border-slate-800">
        <span class="font-mono font-semibold {statusColor(resp.status)}">{resp.status} {resp.statusText}</span>
        <span class="text-slate-500">{resp.timeMs} ms</span>
        <span class="text-slate-500">{humanSize(resp.size)}</span>
        <div class="ml-auto flex items-center gap-1">
          <button class="rounded px-2 py-0.5 {respTab === 'body' ? 'bg-slate-100 text-indigo-600 dark:bg-slate-800 dark:text-indigo-300' : 'text-slate-500'}" onclick={() => (respTab = "body")}>Body</button>
          <button class="rounded px-2 py-0.5 {respTab === 'headers' ? 'bg-slate-100 text-indigo-600 dark:bg-slate-800 dark:text-indigo-300' : 'text-slate-500'}" onclick={() => (respTab = "headers")}>Headers {resp.headers.length}</button>
        </div>
      </div>

      {#if respTab === "body"}
        <div class="flex items-center gap-2 px-3 py-1.5">
          <label class="flex items-center gap-1 text-xs text-slate-500"><input type="checkbox" class="accent-indigo-600" bind:checked={pretty} /> 美化</label>
          <input bind:value={bodySearch} placeholder="搜索响应…" class="{cls.field} w-48 py-1 text-xs" />
          <span class="ml-auto"><CopyButton text={prettyBody} /></span>
        </div>
        <pre class="min-h-0 flex-1 overflow-auto whitespace-pre-wrap break-all px-3 pb-3 font-mono text-xs text-slate-700 dark:text-slate-200">{#each bodySegments as s, i (i)}{#if s.m}<mark class="rounded bg-amber-300/70 text-slate-900">{s.t}</mark>{:else}{s.t}{/if}{/each}</pre>
      {:else}
        <div class="min-h-0 flex-1 overflow-auto px-3 pb-3">
          {#each resp.headers as h, i (i)}
            <div class="flex gap-3 border-b border-slate-100 py-1.5 text-xs dark:border-slate-800">
              <span class="w-56 shrink-0 font-mono font-medium text-indigo-600 dark:text-indigo-300">{h.key}</span>
              <span class="flex-1 select-text break-all font-mono text-slate-700 dark:text-slate-200">{h.value}</span>
            </div>
          {/each}
        </div>
      {/if}
    {:else if !err}
      <div class="flex flex-1 items-center justify-center text-sm text-slate-400">填好请求，点「发送」查看响应。无 CORS 限制，可设任意请求头。</div>
    {/if}
  </div>
</div>

<!-- KV 表格编辑器 -->
{#snippet kvTable(rows: KV[])}
  <div class="space-y-1">
    {#each rows as row, i (i)}
      <div class="flex items-center gap-1.5">
        <input type="checkbox" class="accent-indigo-600" bind:checked={row.on} />
        <input class="{cls.field} min-w-0 flex-1 py-1 font-mono text-xs" placeholder="键" bind:value={row.key} oninput={() => grow(rows)} />
        <input class="{cls.field} min-w-0 flex-1 py-1 font-mono text-xs" placeholder="值" bind:value={row.value} oninput={() => grow(rows)} />
        <button class="rounded p-1 text-slate-400 hover:text-red-500" title="删除" aria-label="删除" onclick={() => removeRow(rows, i)}>✕</button>
      </div>
    {/each}
  </div>
{/snippet}

<!-- 环境变量 -->
{#if showVars}
  <div class="fixed inset-0 z-40 flex items-center justify-center bg-black/40 p-4" role="presentation" onclick={() => (showVars = false)}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="flex max-h-[80vh] w-full max-w-lg flex-col rounded-xl border border-slate-200 bg-white p-5 shadow-xl dark:border-slate-700 dark:bg-slate-900" onclick={(e) => e.stopPropagation()}>
      <h2 class="mb-1 text-base font-semibold text-slate-800 dark:text-slate-100">环境变量</h2>
      <p class="mb-3 text-xs text-slate-400">在 URL / 参数 / 请求头 / Body 里用 <code>{'{{'}<span>名称</span>{'}}'}</code> 引用。发送时替换。</p>
      <div class="min-h-0 flex-1 overflow-y-auto">{@render kvTable(vars)}</div>
      <div class="mt-4 flex justify-end"><button class={cls.btnPrimary} onclick={() => (showVars = false)}>完成</button></div>
    </div>
  </div>
{/if}

<!-- curl 导入/导出 -->
{#if showCurl}
  <div class="fixed inset-0 z-40 flex items-center justify-center bg-black/40 p-4" role="presentation" onclick={() => (showCurl = false)}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="w-full max-w-xl rounded-xl border border-slate-200 bg-white p-5 shadow-xl dark:border-slate-700 dark:bg-slate-900" onclick={(e) => e.stopPropagation()}>
      <h2 class="mb-2 text-base font-semibold text-slate-800 dark:text-slate-100">curl 导入 / 导出</h2>
      <p class="mb-1 text-xs text-slate-400">粘贴 curl 命令后「导入」会填入上方表单；或复制当前请求为 curl。</p>
      <textarea bind:value={curlText} rows="5" placeholder="curl -X POST 'https://...' -H '...' --data-raw '...'" class="{cls.field} w-full font-mono text-xs"></textarea>
      <div class="mt-3 flex items-center justify-between">
        <button class={cls.btnPrimary} onclick={importCurl} disabled={!curlText.trim()}>导入到表单</button>
        <div class="flex items-center gap-2">
          <button class={cls.btn} onclick={() => (curlText = exportCurl())}>生成当前请求的 curl</button>
          <CopyButton text={curlText} />
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- 收藏夹 -->
{#if showSaved}
  <div class="fixed inset-0 z-40 flex items-center justify-center bg-black/40 p-4" role="presentation" onclick={() => (showSaved = false)}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="flex max-h-[80vh] w-full max-w-lg flex-col rounded-xl border border-slate-200 bg-white p-5 shadow-xl dark:border-slate-700 dark:bg-slate-900" onclick={(e) => e.stopPropagation()}>
      <h2 class="mb-2 text-base font-semibold text-slate-800 dark:text-slate-100">收藏夹</h2>
      <div class="min-h-0 flex-1 overflow-y-auto">
        {#each saved as s (s.id)}
          <div class="flex items-center gap-2 rounded-lg px-2 py-1.5 hover:bg-slate-50 dark:hover:bg-slate-800/50">
            <button class="min-w-0 flex-1 text-left" onclick={() => { loadSnapshot(s); showSaved = false; }}>
              <div class="truncate text-sm text-slate-700 dark:text-slate-200">{s.name}</div>
              <div class="truncate font-mono text-[11px] text-slate-400"><span class="font-semibold">{s.method}</span> {s.url}</div>
            </button>
            <button class="rounded p-1 text-slate-400 hover:text-red-500" title="删除" aria-label="删除" onclick={() => (saved = saved.filter((x) => x.id !== s.id))}><Icon name="trash" size={13} /></button>
          </div>
        {/each}
        {#if saved.length === 0}<p class="px-2 py-6 text-center text-xs text-slate-400">还没有收藏。用顶部 📌 收藏当前请求。</p>{/if}
      </div>
      <div class="mt-3 flex justify-end"><button class={cls.btn} onclick={() => (showSaved = false)}>关闭</button></div>
    </div>
  </div>
{/if}

<!-- 历史 -->
{#if showHist}
  <div class="fixed inset-0 z-40 flex items-center justify-center bg-black/40 p-4" role="presentation" onclick={() => (showHist = false)}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="flex max-h-[80vh] w-full max-w-lg flex-col rounded-xl border border-slate-200 bg-white p-5 shadow-xl dark:border-slate-700 dark:bg-slate-900" onclick={(e) => e.stopPropagation()}>
      <div class="mb-2 flex items-center justify-between">
        <h2 class="text-base font-semibold text-slate-800 dark:text-slate-100">历史</h2>
        <button class="{cls.btn} px-2 py-1 text-xs" onclick={() => (history = [])}>清空</button>
      </div>
      <div class="min-h-0 flex-1 overflow-y-auto">
        {#each history as h, i (i)}
          <button class="flex w-full items-center gap-2 rounded-lg px-2 py-1.5 text-left hover:bg-slate-50 dark:hover:bg-slate-800/50" onclick={() => { loadSnapshot(h); showHist = false; }}>
            <span class="font-mono text-[11px] font-semibold {statusColor(h.status)}">{h.status}</span>
            <span class="w-12 shrink-0 font-mono text-[11px] font-semibold text-slate-500">{h.method}</span>
            <span class="min-w-0 flex-1 truncate font-mono text-xs text-slate-700 dark:text-slate-200">{h.url}</span>
            <span class="shrink-0 text-[10px] text-slate-400">{fmtTime(h.at)}</span>
          </button>
        {/each}
        {#if history.length === 0}<p class="px-2 py-6 text-center text-xs text-slate-400">暂无历史。</p>{/if}
      </div>
      <div class="mt-3 flex justify-end"><button class={cls.btn} onclick={() => (showHist = false)}>关闭</button></div>
    </div>
  </div>
{/if}

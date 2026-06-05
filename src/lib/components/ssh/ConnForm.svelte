<script lang="ts">
  import SegmentedControl from "../SegmentedControl.svelte";
  import { cls } from "../../ui";
  import type { ConnView, ConnInput, SecretInput } from "../../ssh";

  let {
    conn = null,
    groups = [],
    onsave,
    oncancel,
  }: {
    conn?: ConnView | null;
    groups?: string[];
    onsave: (conn: ConnInput, secrets?: SecretInput) => void;
    oncancel: () => void;
  } = $props();

  const editing = !!conn;

  let name = $state(conn?.name ?? "");
  let group = $state(conn?.group ?? "");
  let host = $state(conn?.host ?? "");
  let port = $state(conn?.port ?? 22);
  let username = $state(conn?.username ?? "");
  let auth = $state(conn?.auth || "password");
  let note = $state(conn?.note ?? "");
  // 密钥类输入：编辑态留空表示「保持原样」
  let password = $state("");
  let keyPem = $state("");
  let keyPass = $state("");

  function submit() {
    const input: ConnInput = {
      id: conn?.id,
      name: name.trim() || host.trim(),
      group: group.trim(),
      host: host.trim(),
      port: Number(port) || 22,
      username: username.trim(),
      auth,
      note,
    };
    // 仅把用户填了的密码类字段下发；留空 = 保持原样
    const secrets: SecretInput = {};
    if (password) secrets.password = password;
    if (keyPem) secrets.keyPem = keyPem;
    if (keyPass) secrets.keyPass = keyPass;
    const hasSecret = Object.keys(secrets).length > 0;
    onsave(input, hasSecret ? secrets : undefined);
  }
</script>

<div class="fixed inset-0 z-30 flex items-center justify-center bg-black/40 p-4" role="presentation" onclick={oncancel}>
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="max-h-[90vh] w-full max-w-lg overflow-y-auto rounded-xl border border-slate-200 bg-white p-5 shadow-xl dark:border-slate-700 dark:bg-slate-900"
    onclick={(e) => e.stopPropagation()}
  >
    <h2 class="mb-4 text-base font-semibold text-slate-800 dark:text-slate-100">
      {editing ? "编辑连接" : "新建连接"}
    </h2>

    <div class="space-y-3">
      <div class="grid grid-cols-2 gap-3">
        <label class="block">
          <span class="{cls.label} mb-1 block">名称</span>
          <input bind:value={name} placeholder="如 web-01" class={cls.field} />
        </label>
        <label class="block">
          <span class="{cls.label} mb-1 block">分组</span>
          <input bind:value={group} list="ssh-groups" placeholder="如 生产" class={cls.field} />
          <datalist id="ssh-groups">
            {#each groups as g (g)}<option value={g}></option>{/each}
          </datalist>
        </label>
      </div>

      <div class="grid grid-cols-3 gap-3">
        <label class="col-span-2 block">
          <span class="{cls.label} mb-1 block">主机</span>
          <input bind:value={host} placeholder="10.0.0.1 / example.com" class="{cls.field} font-mono" />
        </label>
        <label class="block">
          <span class="{cls.label} mb-1 block">端口</span>
          <input type="number" bind:value={port} class="{cls.field} font-mono" />
        </label>
      </div>

      <label class="block">
        <span class="{cls.label} mb-1 block">用户名</span>
        <input bind:value={username} placeholder="root" class="{cls.field} font-mono" />
      </label>

      <div>
        <span class="{cls.label} mb-1 block">认证方式</span>
        <SegmentedControl
          bind:value={auth}
          options={[
            { label: "密码", value: "password" },
            { label: "私钥", value: "key" },
          ]}
        />
      </div>

      {#if auth === "password"}
        <label class="block">
          <span class="{cls.label} mb-1 block">密码{editing ? "（留空＝不修改）" : ""}</span>
          <input type="password" bind:value={password} class="{cls.field} font-mono" />
        </label>
      {:else}
        <label class="block">
          <span class="{cls.label} mb-1 block">私钥 PEM{editing ? "（留空＝不修改）" : ""}</span>
          <textarea
            bind:value={keyPem}
            spellcheck="false"
            placeholder="-----BEGIN OPENSSH PRIVATE KEY-----"
            class="{cls.field} h-28 resize-none font-mono text-xs"
          ></textarea>
        </label>
        <label class="block">
          <span class="{cls.label} mb-1 block">私钥口令（若有）</span>
          <input type="password" bind:value={keyPass} class="{cls.field} font-mono" />
        </label>
      {/if}

      <label class="block">
        <span class="{cls.label} mb-1 block">备注</span>
        <input bind:value={note} class={cls.field} />
      </label>
    </div>

    <p class="mt-3 text-xs text-slate-400">
      密码 / 私钥会用主密码加密后存储；其余字段明文。保存含密码的连接需先解锁主密码。
    </p>

    <div class="mt-4 flex justify-end gap-2">
      <button class={cls.btn} onclick={oncancel}>取消</button>
      <button class={cls.btnPrimary} onclick={submit} disabled={!host.trim() || !username.trim()}>保存</button>
    </div>
  </div>
</div>

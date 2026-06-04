<script lang="ts">
  import { api, errMsg } from "../ipc";
  import ToolPanel from "../components/ToolPanel.svelte";
  import SegmentedControl from "../components/SegmentedControl.svelte";
  import CopyButton from "../components/CopyButton.svelte";
  import RsaKeygen from "../components/RsaKeygen.svelte";
  import { cls } from "../ui";

  let algo = $state("aes-cbc");
  let direction = $state("encrypt");
  let keyMode = $state("passphrase");
  // 各模式独立密钥：仅口令预填默认，Hex/Base64 留空
  let keyPass = $state("5qEBzrSccgOyWCSk");
  let keyHex = $state("");
  let keyB64 = $state("");
  let publicPem = $state("");
  let privatePem = $state("");
  let input = $state("");
  let output = $state("");
  let error = $state("");
  let busy = $state(false);

  const isRsa = $derived(algo === "rsa");
  const encrypting = $derived(direction === "encrypt");

  async function run() {
    busy = true;
    error = "";
    output = "";
    try {
      const symKey = keyMode === "passphrase" ? keyPass : keyMode === "hex" ? keyHex : keyB64;
      const useKey = isRsa ? (encrypting ? publicPem : privatePem) : symKey;
      output = await api.cryptoProcess(algo, direction, keyMode, useKey, input);
    } catch (e) {
      error = errMsg(e);
    } finally {
      busy = false;
    }
  }

</script>

<ToolPanel
  title="加密 / 解密"
  description="对称 AES-256-GCM / AES-256-CBC / ChaCha20-Poly1305（口令或原始密钥）+ 非对称 RSA-OAEP。"
>
  <div class="mb-3 flex flex-wrap items-center gap-3">
    <SegmentedControl
      bind:value={algo}
      options={[
        { label: "AES-CBC", value: "aes-cbc" },
        { label: "AES-GCM", value: "aes-gcm" },
        { label: "ChaCha20", value: "chacha20" },
        { label: "RSA", value: "rsa" },
      ]}
    />
    <SegmentedControl
      bind:value={direction}
      options={[
        { label: "加密", value: "encrypt" },
        { label: "解密", value: "decrypt" },
      ]}
    />
  </div>

  {#if !isRsa}
    <div class="space-y-2">
      <div class="flex flex-wrap items-center gap-3">
        <span class="{cls.label} w-14 shrink-0">密钥</span>
        <SegmentedControl
          bind:value={keyMode}
          options={[
            { label: "口令", value: "passphrase" },
            { label: "Hex", value: "hex" },
            { label: "Base64", value: "base64" },
          ]}
        />
      </div>
      {#if keyMode === "passphrase"}
        <input bind:value={keyPass} placeholder="输入口令（原始字节不足 32 补零、超过截断作密钥）" class="{cls.field} font-mono" />
      {:else if keyMode === "hex"}
        <input bind:value={keyHex} placeholder="32 字节密钥的 Hex（64 个十六进制字符）" class="{cls.field} font-mono" />
      {:else}
        <input bind:value={keyB64} placeholder="32 字节密钥的 Base64" class="{cls.field} font-mono" />
      {/if}
    </div>
  {:else}
    <div class="{cls.card} space-y-2 p-3">
      <div class="text-xs text-slate-400">
        加密用公钥、解密用私钥。可点「生成密钥对」，或粘贴自有 PEM（也可用侧栏「RSA 密钥对」工具生成）。
      </div>
      <RsaKeygen bind:publicPem bind:privatePem />
    </div>
  {/if}

  <div class="mt-3 grid grid-cols-1 gap-4 lg:grid-cols-2">
    <div>
      <div class="mb-1 flex h-7 items-center">
        <span class={cls.label}>{encrypting ? "明文" : "密文（Base64）"}</span>
      </div>
      <textarea
        bind:value={input}
        spellcheck="false"
        class="{cls.field} h-44 resize-none font-mono leading-relaxed"
      ></textarea>
    </div>
    <div>
      <div class="mb-1 flex h-7 items-center justify-between">
        <span class={cls.label}>{encrypting ? "密文（Base64）" : "明文"}</span>
        {#if output}<CopyButton text={output} />{/if}
      </div>
      <textarea
        value={output}
        readonly
        spellcheck="false"
        class="{cls.field} h-44 resize-none bg-slate-50 font-mono leading-relaxed dark:bg-slate-900/60"
      ></textarea>
    </div>
  </div>

  <div class="mt-3 flex items-center gap-3">
    <button class={cls.btnPrimary} onclick={run} disabled={busy}>
      {busy ? "处理中…" : encrypting ? "加密" : "解密"}
    </button>
    {#if error}
      <span class="rounded-md bg-red-50 px-2 py-1 text-sm text-red-600 dark:bg-red-950/40 dark:text-red-400">{error}</span>
    {/if}
  </div>
</ToolPanel>

<script lang="ts">
  import { api, errMsg } from "../ipc";
  import SegmentedControl from "./SegmentedControl.svelte";
  import CopyButton from "./CopyButton.svelte";
  import { cls } from "../ui";

  // 可复用的 RSA 密钥对生成：公私钥通过 $bindable 暴露，父组件可读取。
  let {
    publicPem = $bindable(""),
    privatePem = $bindable(""),
  }: { publicPem?: string; privatePem?: string } = $props();

  let bits = $state(2048);
  let busy = $state(false);
  let error = $state("");

  async function gen() {
    busy = true;
    error = "";
    try {
      const kp = await api.rsaGenerate(bits);
      publicPem = kp.public_pem;
      privatePem = kp.private_pem;
    } catch (e) {
      error = errMsg(e);
    } finally {
      busy = false;
    }
  }
</script>

<div class="space-y-2">
  <div class="flex flex-wrap items-center gap-3">
    <button class={cls.btnPrimary} onclick={gen} disabled={busy}>
      {busy ? "生成中…" : "生成密钥对"}
    </button>
    <SegmentedControl
      bind:value={bits}
      options={[
        { label: "2048", value: 2048 },
        { label: "3072", value: 3072 },
        { label: "4096", value: 4096 },
      ]}
    />
    {#if error}
      <span class="rounded-md bg-red-50 px-2 py-1 text-sm text-red-600 dark:bg-red-950/40 dark:text-red-400">{error}</span>
    {/if}
  </div>
  <div class="grid grid-cols-1 gap-3 lg:grid-cols-2">
    <div>
      <div class="mb-1 flex items-center justify-between">
        <span class={cls.label}>公钥 PEM (SPKI)</span>
        {#if publicPem}<CopyButton text={publicPem} />{/if}
      </div>
      <textarea bind:value={publicPem} spellcheck="false" class="{cls.field} h-36 resize-none font-mono text-xs"></textarea>
    </div>
    <div>
      <div class="mb-1 flex items-center justify-between">
        <span class={cls.label}>私钥 PEM (PKCS#8)</span>
        {#if privatePem}<CopyButton text={privatePem} />{/if}
      </div>
      <textarea bind:value={privatePem} spellcheck="false" class="{cls.field} h-36 resize-none font-mono text-xs"></textarea>
    </div>
  </div>
</div>

<script lang="ts">
  import ToolPanel from "../components/ToolPanel.svelte";
  import RsaKeygen from "../components/RsaKeygen.svelte";
</script>

<ToolPanel
  title="RSA 密钥对"
  description="生成 RSA 公私钥对（2048 / 3072 / 4096 位），公钥 SPKI、私钥 PKCS#8 PEM。可直接复制到「加密 / 解密」工具使用。"
>
  <RsaKeygen />
</ToolPanel>

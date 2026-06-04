<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../ipc";
  import ComboBox from "./ComboBox.svelte";

  // 时区选择：复用自绘下拉，严格模式（只接受合法 IANA 时区，否则还原）。
  let { value = $bindable() }: { value: string } = $props();
  let list = $state<string[]>([]);

  onMount(async () => {
    try {
      list = await api.listTimezones();
    } catch {}
  });
</script>

<ComboBox bind:value options={list} strict placeholder="点击选择时区…" />

<script lang="ts">
  // 轻量内联 SVG 图标（stroke 用 currentColor），无第三方图标库依赖。
  let {
    name,
    size = 20,
    class: cls = "",
  }: { name: string; size?: number; class?: string } = $props();
</script>

<svg
  xmlns="http://www.w3.org/2000/svg"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="2"
  stroke-linecap="round"
  stroke-linejoin="round"
  class={cls}
  aria-hidden="true"
>
  {#if name === "clock"}
    <circle cx="12" cy="12" r="9" />
    <path d="M12 7v5l3 2" />
  {:else if name === "chip"}
    <rect x="6" y="6" width="12" height="12" rx="2" />
    <rect x="9.5" y="9.5" width="5" height="5" rx="1" />
    <path d="M9 1v3M15 1v3M9 20v3M15 20v3M1 9h3M1 15h3M20 9h3M20 15h3" />
  {:else if name === "swap"}
    <path d="M16 3l4 4-4 4" />
    <path d="M20 7H8" />
    <path d="M8 21l-4-4 4-4" />
    <path d="M4 17h12" />
  {:else if name === "braces"}
    <path d="M7 4a2 2 0 0 0-2 2v3a2 2 0 0 1-2 2 2 2 0 0 1 2 2v3a2 2 0 0 0 2 2" />
    <path d="M17 4a2 2 0 0 1 2 2v3a2 2 0 0 0 2 2 2 2 0 0 0-2 2v3a2 2 0 0 1-2 2" />
  {:else if name === "code"}
    <path d="M16 18l6-6-6-6" />
    <path d="M8 6l-6 6 6 6" />
  {:else if name === "hash"}
    <path d="M4 9h16M4 15h16M10 3 8 21M16 3l-2 18" />
  {:else if name === "copy"}
    <rect x="8" y="8" width="13" height="13" rx="2" />
    <path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2" />
  {:else if name === "check"}
    <path d="M20 6 9 17l-5-5" />
  {:else if name === "sun"}
    <circle cx="12" cy="12" r="4" />
    <path d="M12 2v2M12 20v2M4 12H2M22 12h-2M5 5l1.5 1.5M17.5 17.5 19 19M5 19l1.5-1.5M17.5 6.5 19 5" />
  {:else if name === "moon"}
    <path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" />
  {:else if name === "search"}
    <circle cx="11" cy="11" r="7" />
    <path d="M21 21l-4.3-4.3" />
  {:else if name === "refresh"}
    <path d="M21 12a9 9 0 1 1-2.6-6.4" />
    <path d="M21 3v6h-6" />
  {:else if name === "calendar"}
    <rect x="3" y="4" width="18" height="17" rx="2" />
    <path d="M3 9h18M8 2v4M16 2v4" />
  {:else if name === "asterisk"}
    <path d="M12 4v16M4.5 7.5l15 9M19.5 7.5l-15 9" />
  {:else if name === "network"}
    <rect x="9" y="3" width="6" height="5" rx="1" />
    <rect x="3" y="16" width="6" height="5" rx="1" />
    <rect x="15" y="16" width="6" height="5" rx="1" />
    <path d="M12 8v4M6 16v-2h12v2" />
  {:else if name === "filecode"}
    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
    <path d="M14 2v6h6" />
    <path d="M9 13l-2 2 2 2M15 13l2 2-2 2" />
  {:else if name === "type"}
    <path d="M4 6V4h16v2M12 4v16M9 20h6" />
  {:else if name === "columns"}
    <rect x="3" y="4" width="7" height="16" rx="1" />
    <rect x="14" y="4" width="7" height="16" rx="1" />
  {:else if name === "globe"}
    <circle cx="12" cy="12" r="9" />
    <path d="M3 12h18M12 3a15 15 0 0 1 0 18M12 3a15 15 0 0 0 0 18" />
  {:else if name === "server"}
    <rect x="3" y="4" width="18" height="7" rx="1" />
    <rect x="3" y="13" width="18" height="7" rx="1" />
    <path d="M7 7.5h.01M7 16.5h.01" />
  {:else if name === "monitor"}
    <rect x="3" y="4" width="18" height="12" rx="2" />
    <path d="M8 20h8M12 16v4" />
  {:else if name === "key"}
    <circle cx="7.5" cy="15.5" r="5.5" />
    <path d="m21 2-9.6 9.6" />
    <path d="m15.5 7.5 3 3L22 7l-3-3" />
  {:else if name === "lock"}
    <rect x="3" y="11" width="18" height="11" rx="2" />
    <path d="M7 11V7a5 5 0 0 1 10 0v4" />
  {:else if name === "shuffle"}
    <path d="M16 3h5v5" />
    <path d="M4 20 21 3" />
    <path d="M21 16v5h-5" />
    <path d="m15 15 6 6" />
    <path d="M4 4l5 5" />
  {:else if name === "languages"}
    <path d="m5 8 6 6" />
    <path d="m4 14 6-6 2-3" />
    <path d="M2 5h12" />
    <path d="M7 2h1" />
    <path d="m22 22-5-10-5 10" />
    <path d="M14 18h6" />
  {:else if name === "terminal"}
    <path d="m4 17 6-6-6-6" />
    <path d="M12 19h8" />
  {:else if name === "trash"}
    <path d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
  {:else if name === "folder"}
    <path d="M4 20a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h5l2 3h7a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2z" />
  {:else if name === "upload"}
    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M17 8l-5-5-5 5M12 3v12" />
  {:else if name === "download"}
    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4M7 10l5 5 5-5M12 15V3" />
  {:else if name === "pin"}
    <path d="M12 17v5" />
    <path d="M9 10.8V4h6v6.8l2.2 2.4a1 1 0 0 1-.7 1.8H7.5a1 1 0 0 1-.7-1.8z" />
  {:else if name === "pencil"}
    <path d="M12 20h9" />
    <path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4z" />
  {:else if name === "chevron-left"}
    <path d="M15 18l-6-6 6-6" />
  {:else if name === "plus"}
    <path d="M12 5v14M5 12h14" />
  {:else if name === "unlock"}
    <rect x="3" y="11" width="18" height="11" rx="2" />
    <path d="M7 11V7a5 5 0 0 1 9.9-1" />
  {/if}
</svg>

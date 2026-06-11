// 共享的 Tailwind 类名片段，避免在各组件里重复书写（DRY，无需 @apply）。
export const cls = {
  field:
    "w-full rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-3 py-2 text-sm text-slate-800 dark:text-slate-100 outline-none transition focus:border-indigo-500 focus:ring-2 focus:ring-indigo-500/30 placeholder:text-slate-400",
  card:
    "rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/40",
  label: "text-xs font-medium text-slate-500 dark:text-slate-400",
  // 大文本框高度：同时跟随「窗口大小」和「页面缩放」，并带上下限。
  // 用 calc(vh + rem)：vh 段随窗口拖拽/全屏变化（zoom 不变），rem 段随页面缩放
  // （Ctrl/Cmd +/-）变化（窗口不变）——两者相加，缩放和拖窗都能让框变大变小；
  // clamp 的上下限也用 rem，故即便顶到边界，缩放仍会改变实际高度。
  // 三档按同一比例统一收窄：editorTall 在 1080p 全屏约占屏高 60%（42vh+12rem≈646px），
  // rem 段保证缩放仍有效。editorMid 用于下方还有结果面板的输入框，按比例压低一档；
  // editorShort 用于下方还有「必须能点到」的动作按钮（如加解密），vh 系数最低，
  // 保证非全屏窗口（约 650px 高起）整页不滚动、按钮不被挤出屏幕。
  editorTall: "h-[clamp(15rem,calc(42vh_+_12rem),68rem)]",
  editorMid: "h-[clamp(10rem,calc(33vh_+_9rem),48rem)]",
  editorShort: "h-[clamp(8rem,calc(22vh_+_5rem),28rem)]",
  btn:
    "inline-flex items-center justify-center gap-1.5 rounded-lg px-3 py-2 text-sm font-medium transition border border-slate-200 dark:border-slate-700 text-slate-700 dark:text-slate-200 hover:bg-slate-50 dark:hover:bg-slate-800 disabled:opacity-40",
  btnPrimary:
    "inline-flex items-center justify-center gap-1.5 rounded-lg px-3 py-2 text-sm font-medium transition bg-indigo-600 hover:bg-indigo-500 text-white disabled:opacity-40",
};

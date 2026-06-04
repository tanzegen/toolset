// 共享的 Tailwind 类名片段，避免在各组件里重复书写（DRY，无需 @apply）。
export const cls = {
  field:
    "w-full rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 px-3 py-2 text-sm text-slate-800 dark:text-slate-100 outline-none transition focus:border-indigo-500 focus:ring-2 focus:ring-indigo-500/30 placeholder:text-slate-400",
  card:
    "rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/40",
  label: "text-xs font-medium text-slate-500 dark:text-slate-400",
  btn:
    "inline-flex items-center justify-center gap-1.5 rounded-lg px-3 py-2 text-sm font-medium transition border border-slate-200 dark:border-slate-700 text-slate-700 dark:text-slate-200 hover:bg-slate-50 dark:hover:bg-slate-800 disabled:opacity-40",
  btnPrimary:
    "inline-flex items-center justify-center gap-1.5 rounded-lg px-3 py-2 text-sm font-medium transition bg-indigo-600 hover:bg-indigo-500 text-white disabled:opacity-40",
};

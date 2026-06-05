// 工具内容跨重启持久化（localStorage）。在组件初始化阶段调用一次。
//
// - 同步读取已保存快照，交给 load() 回写到各 $state；这一步在 $effect 注册前完成，
//   因此恢复的值不会被组件默认值覆盖。
// - 注册一个 $effect 跟踪 save() 读取到的字段，任意变化即写回 localStorage。
//
// 安全约定：只持久化用户输入与选项。敏感的“生成结果”——随机密码、RSA 私钥、
// 密文与对称密钥本身——一律不传入本函数，避免明文落盘。

import { browser } from "$app/environment";

const PREFIX = "toolset:tool:";

export function persist<T extends Record<string, unknown>>(
  key: string,
  opts: { save: () => T; load: (saved: Partial<T>) => void },
): boolean {
  if (!browser) return false;
  const storageKey = PREFIX + key;
  let loaded = false;
  const raw = localStorage.getItem(storageKey);
  if (raw) {
    try {
      opts.load(JSON.parse(raw) as Partial<T>);
      loaded = true;
    } catch {
      // 快照损坏：忽略并沿用默认值
    }
  }
  $effect(() => {
    localStorage.setItem(storageKey, JSON.stringify(opts.save()));
  });
  return loaded;
}

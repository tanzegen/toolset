// JSON 字段树：从一段 JSON（对象或对象数组）推断可勾选的字段路径，供「字段提取」用。
// 路径语义与后端 json_pick 一致：路径是一串键，数组「透明」逐元素合并/投影，路径不含下标。

export type FieldNode = {
  key: string; // 本级键名
  path: string; // 完整点路径，如 addr.city
  type: string; // 展示用类型标签
  sample: string; // 叶子节点的首个示例值（非叶为空）
  children: FieldNode[]; // 空 = 叶子
};

/** 把值「拆」到对象层：数组透明展开到元素，对象原样，标量返回空。 */
function collectObjects(v: unknown): Record<string, unknown>[] {
  if (Array.isArray(v)) return v.flatMap(collectObjects);
  if (v && typeof v === "object") return [v as Record<string, unknown>];
  return [];
}

function typeLabel(vals: unknown[]): string {
  const kinds = new Set<string>();
  for (const v of vals) {
    if (Array.isArray(v)) kinds.add(collectObjects(v).length ? "数组<对象>" : "数组");
    else if (v === null) kinds.add("null");
    else if (typeof v === "object") kinds.add("对象");
    else kinds.add(typeof v); // string / number / boolean
  }
  return kinds.size === 1 ? [...kinds][0] : "混合";
}

function sampleOf(vals: unknown[]): string {
  for (const v of vals) {
    if (v !== null && typeof v !== "object") {
      const s = JSON.stringify(v);
      return s.length > 24 ? s.slice(0, 24) + "…" : s;
    }
  }
  return "";
}

/** 对一组「同一位置」的候选值（跨数组元素并集）构建字段节点。 */
function buildNodes(vals: unknown[], parent: string): FieldNode[] {
  const objs = vals.flatMap(collectObjects);
  if (!objs.length) return [];
  const keys: string[] = [];
  for (const o of objs) for (const k of Object.keys(o)) if (!keys.includes(k)) keys.push(k);
  return keys.map((k) => {
    const cv = objs.filter((o) => k in o).map((o) => o[k]);
    const path = parent ? `${parent}.${k}` : k;
    const children = buildNodes(cv, path);
    return { key: k, path, type: typeLabel(cv), sample: children.length ? "" : sampleOf(cv), children };
  });
}

/** 从根值（对象或数组）推断字段树。 */
export function buildFieldTree(root: unknown): FieldNode[] {
  return buildNodes([root], "");
}

/** 节点下所有叶子路径（用于三态勾选与全选）。 */
export function leafPaths(n: FieldNode): string[] {
  return n.children.length ? n.children.flatMap(leafPaths) : [n.path];
}

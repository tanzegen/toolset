// 拖拽改宽的 Svelte action：把手元素按下后跟随指针，持续回调 clientX。
// 调用方据自身左边缘把 clientX 换算成新宽度并做夹取/持久化。
// 用 Pointer Capture，拖动中改 body 光标/禁选，松开复原。

export interface ResizeOpts {
  onmove: (clientX: number) => void;
  onstart?: () => void;
  onend?: () => void;
}

export function resizeHandle(node: HTMLElement, opts: ResizeOpts) {
  let o = opts;

  function down(e: PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    o.onstart?.();
    const move = (ev: PointerEvent) => o.onmove(ev.clientX);
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      o.onend?.();
    };
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
  }

  node.addEventListener("pointerdown", down);
  return {
    update(next: ResizeOpts) {
      o = next;
    },
    destroy() {
      node.removeEventListener("pointerdown", down);
    },
  };
}

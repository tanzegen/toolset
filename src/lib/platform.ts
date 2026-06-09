// 运行平台判定：WebView 内通过 navigator 推断，避免为这点小事引入 Tauri os 插件与权限。
const plat = typeof navigator !== "undefined" ? navigator.platform : "";
const ua = typeof navigator !== "undefined" ? navigator.userAgent : "";

export const IS_MAC = /Mac|iPhone|iPad/i.test(plat) || /Mac OS X/i.test(ua);
export const IS_WINDOWS = /Win/i.test(plat) || /Windows/i.test(ua);

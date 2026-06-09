#!/bin/bash
#
# 让「未公证」版 Toolset 在 macOS 上能打开的临时脚本。
# 做两件事：① 解除下载隔离属性(com.apple.quarantine)  ② 本地 ad-hoc 重签名(Apple Silicon 必需)。
# 等正式的 Developer ID 公证版发布后，本脚本即不再需要。
#
# 用法（任选其一）：
#   • 把 Toolset.app 拖进「应用程序」，然后在「终端」执行：
#       bash macos-fix-open.command
#   • 指定路径：
#       bash macos-fix-open.command "/Applications/Toolset.app"
#   • 或在「访达」里双击本文件（首次需右键 → 打开）。

APP="${1:-}"

# 未传参数则按常见位置自动查找 Toolset.app
if [ -z "$APP" ]; then
  for p in \
    "/Applications/Toolset.app" \
    "$HOME/Applications/Toolset.app" \
    "$HOME/Downloads/Toolset.app" \
    "$HOME/Desktop/Toolset.app"; do
    if [ -d "$p" ]; then APP="$p"; break; fi
  done
fi

# 仍未找到则让用户把 App 拖进窗口
if [ -z "$APP" ] || [ ! -d "$APP" ]; then
  echo "未找到 Toolset.app。请把它拖进本窗口后回车（或先拖进「应用程序」再重跑）。"
  read -r -p "Toolset.app 路径: " APP
  APP="$(echo "$APP" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')"
fi

if [ ! -d "$APP" ]; then
  echo "✗ 路径无效：$APP"
  read -r -p "按回车关闭…" _ 2>/dev/null
  exit 1
fi

echo "→ 目标：$APP"

echo "→ 解除下载隔离 (com.apple.quarantine)…"
xattr -dr com.apple.quarantine "$APP" 2>/dev/null

echo "→ 本地 ad-hoc 重签名…"
if ! codesign --force --deep --sign - "$APP" 2>/dev/null; then
  codesign --force --sign - "$APP" 2>/dev/null
fi

echo ""
echo "✓ 完成。现在去「应用程序」或原位置双击打开 Toolset 即可。"
echo "  若仍被拦截：系统设置 → 隐私与安全性 → 点「仍要打开」。"
read -r -p "按回车关闭…" _ 2>/dev/null

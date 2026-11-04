#!/bin/sh
# mdview 安装脚本：从 GitHub Release 取本平台产物，装进 PREFIX（默认 /usr/local/bin）。
#   curl -fsSL https://raw.githubusercontent.com/pengchangg/mdview/main/install.sh | sh
#   MDVIEW_PREFIX=~/.local/bin MDVIEW_VERSION=0.4.0 sh install.sh
# 代理走标准 https_proxy 环境变量，curl 自己会读。
set -eu

repo="pengchangg/mdview"
prefix="${MDVIEW_PREFIX:-/usr/local/bin}"

os="$(uname -s)"
case "$os" in
Linux) plat="unknown-linux-gnu" ext="" ;;
Darwin) plat="apple-darwin" ext="" ;;
MINGW* | MSYS* | CYGWIN*) plat="pc-windows-msvc" ext=".exe" ;;
*)
    echo "mdview: 不支持的系统 $os（支持 Linux / macOS / Windows），见 README 的 Install" >&2
    exit 1
    ;;
esac

arch="$(uname -m)"
case "$arch" in
x86_64 | amd64) cpu="x86_64" ;;
aarch64 | arm64) cpu="aarch64" ;;
*)
    echo "mdview: 不支持的架构 $arch（支持 x86_64 / aarch64）" >&2
    exit 1
    ;;
esac

# 只发了 Apple Silicon 的 mac 包；Intel mac 用户走 cargo，不给他们一个 404。
if [ "$os" = Darwin ] && [ "$cpu" = x86_64 ]; then
    echo "mdview: 没有 Intel macOS 的构建，请用 cargo install mdview-rs" >&2
    exit 1
fi

target="$cpu-$plat"

# 版本号没给就查 latest release 的 tag；api.github.com 匿名限流 60 次/小时，
# 撞上限流就显式传 MDVIEW_VERSION。
ver="${MDVIEW_VERSION:-}"
if [ -z "$ver" ]; then
    ver="$(curl -fsSL https://api.github.com/repos/$repo/releases/latest |
        grep '"tag_name"' | head -n1 | cut -d'"' -f4)" || ver=""
    if [ -z "$ver" ]; then
        echo "mdview: 取不到最新版本号（GitHub API 限流？），请显式设 MDVIEW_VERSION=0.4.0" >&2
        exit 1
    fi
fi
ver="${ver#v}"

url="https://github.com/$repo/releases/download/v$ver/mdview-$target.tar.gz"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "==> $url"
curl -fL --retry 3 --retry-delay 2 -o "$tmp/mdview.tar.gz" "$url" || {
    echo "mdview: 下载失败 v$ver $target —— 这个版本有这个平台的包吗" >&2
    exit 1
}
tar -xzf "$tmp/mdview.tar.gz" -C "$tmp"

if [ -w "$prefix" ]; then
    mkdir -p "$prefix"
elif [ -n "${SUDO_PROMPT:-}" ] || command -v sudo >/dev/null 2>&1; then
    sudo mkdir -p "$prefix"
    install_cmd="sudo install"
else
    echo "mdview: $prefix 不可写，也没有 sudo；换个 MDVIEW_PREFIX" >&2
    exit 1
fi
${install_cmd:-install} -m 755 "$tmp/mdview$ext" "$prefix/mdview$ext"

echo "==> 已装 $prefix/mdview$ext（v$ver, $target）"
# shortcut: 只认 glibc 的包，musl/alpine 用户出现时再加交叉构建。
echo "    PATH 里没有的话：export PATH=\"$prefix:\$PATH\""
echo "    补全：mdview --print-completion bash > ~/.local/share/bash-completion/completions/mdview"

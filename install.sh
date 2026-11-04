#!/bin/sh
# mdview installer: grabs this platform's release archive and installs it into PREFIX (/usr/local/bin).
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
    echo "mdview: unsupported OS $os (Linux / macOS / Windows); see README Install" >&2
    exit 1
    ;;
esac

arch="$(uname -m)"
case "$arch" in
x86_64 | amd64) cpu="x86_64" ;;
aarch64 | arm64) cpu="aarch64" ;;
*)
    echo "mdview: unsupported architecture $arch (x86_64 / aarch64)" >&2
    exit 1
    ;;
esac

# only an Apple Silicon mac archive is published; Intel mac users go through cargo, not a 404.
if [ "$os" = Darwin ] && [ "$cpu" = x86_64 ]; then
    echo "mdview: no Intel macOS build is published; use cargo install mdview-rs" >&2
    exit 1
fi

target="$cpu-$plat"

# no version given -> read the latest release tag; api.github.com is rate-limited to 60 anonymous
# requests/hour, so pass MDVIEW_VERSION if you hit it.
ver="${MDVIEW_VERSION:-}"
if [ -z "$ver" ]; then
    ver="$(curl -fsSL https://api.github.com/repos/$repo/releases/latest |
        grep '"tag_name"' | head -n1 | cut -d'"' -f4)" || ver=""
    if [ -z "$ver" ]; then
        echo "mdview: cannot read the latest version (GitHub API rate limit?); set MDVIEW_VERSION=0.4.0" >&2
        exit 1
    fi
fi
ver="${ver#v}"

url="https://github.com/$repo/releases/download/v$ver/mdview-$target.tar.gz"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "==> $url"
curl -fL --retry 3 --retry-delay 2 -o "$tmp/mdview.tar.gz" "$url" || {
    echo "mdview: download failed for v$ver $target - is there an archive for this platform?" >&2
    exit 1
}
tar -xzf "$tmp/mdview.tar.gz" -C "$tmp"

if [ -w "$prefix" ]; then
    mkdir -p "$prefix"
elif [ -n "${SUDO_PROMPT:-}" ] || command -v sudo >/dev/null 2>&1; then
    sudo mkdir -p "$prefix"
    install_cmd="sudo install"
else
    echo "mdview: $prefix is not writable and there is no sudo; pick another MDVIEW_PREFIX" >&2
    exit 1
fi
${install_cmd:-install} -m 755 "$tmp/mdview$ext" "$prefix/mdview$ext"

echo "==> installed $prefix/mdview$ext (v$ver, $target)"
# shortcut: 只认 glibc 的包，musl/alpine 用户出现时再加交叉构建。
echo "    not on PATH? export PATH=\"$prefix:\$PATH\""
echo "    completions: mdview --print-completion bash > ~/.local/share/bash-completion/completions/mdview"

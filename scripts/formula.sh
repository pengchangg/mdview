#!/usr/bin/env bash
# 生成 Formula/mdview.rb：算出各平台 tarball 的 sha256 写进 brew formula。
# Homebrew 要的是**已发布**产物的哈希，所以这步只能在 tag 之后跑 —— CI 的 tap job 在 release
# 之后自动跑本脚本（产物就是刚下的 artifact，走 MDVIEW_ARCHIVE_DIR）并把结果提交回 main。
# 手动补一次仍然用它：
#
# 用法: scripts/formula.sh 0.4.0                              # 从 release 页面下载算哈希
#       MDVIEW_ARCHIVE_DIR=artifacts scripts/formula.sh 0.4.0 # 用已经下好的 tarball
set -euo pipefail

ver="${1:-}"
if ! [[ "$ver" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
    echo "用法: scripts/formula.sh <version>，例如 0.4.0" >&2
    exit 1
fi

cd "$(dirname "$0")/.."
repo="pengchangg/mdview"
out="Formula/mdview.rb"

# 每个系统一个块，里面按 CPU 分支。Intel mac 没有产物（CI 只出 aarch64-apple-darwin），
# 那个平台走 cargo install mdview-rs。
rows=(
    "on_linux|on_intel|x86_64-unknown-linux-gnu"
    "on_linux|on_arm|aarch64-unknown-linux-gnu"
    "on_macos|on_arm|aarch64-apple-darwin"
)

body=""
missing=()
for outer in on_linux on_macos; do
    inner_body=""
    for row in "${rows[@]}"; do
        IFS='|' read -r o inner target <<<"$row"
        [ "$o" = "$outer" ] || continue
        url="https://github.com/$repo/releases/download/v$ver/mdview-$target.tar.gz"
        if [ -n "${MDVIEW_ARCHIVE_DIR:-}" ]; then
            # CI 里 tarball 已在本地（download-artifact 按 artifact 名分了一层），别再下一遍。
            file="$(find "$MDVIEW_ARCHIVE_DIR" -name "mdview-$target.tar.gz" -print -quit 2>/dev/null)" || file=""
            if [ -z "$file" ]; then
                missing+=("$target")
                continue
            fi
            hash="$(sha256sum "$file" | cut -d' ' -f1)"
        elif ! hash="$(curl -fsSL --retry 3 --retry-delay 2 "$url" | sha256sum | cut -d' ' -f1)"; then
            # -f：404 时不落盘，好区分「这个版本还没出这个平台」。代理照常走 https_proxy 环境变量。
            missing+=("$target")
            continue
        fi
        inner_body+="    $inner do
      url \"$url\"
      sha256 \"$hash\"
    end
"
    done
    [ -n "$inner_body" ] || continue
    body+="  $outer do
$inner_body  end

"
done

if [ -z "$body" ]; then
    echo "v$ver 一个 tarball 都没取到：tag 推了吗，CI 的 build 跑完了吗" >&2
    exit 1
fi
if [ ${#missing[@]} -gt 0 ]; then
    echo "跳过的平台（该版本没有这个 tarball）: ${missing[*]}" >&2
fi

mkdir -p "$(dirname "$out")"
{
    echo "# 由 scripts/formula.sh $ver 生成（发布后由 CI 的 tap job 跑），别手改：sha256 来自已发布的 tarball。"
    echo "class Mdview < Formula"
    echo '  desc "Terminal Markdown viewer: scroll, search, outline, mermaid diagrams, live reload"'
    echo "  homepage \"https://github.com/$repo\""
    echo "  version \"$ver\""
    echo '  license "MIT"'
    echo
    printf '%s' "$body"
    echo '  def install'
    echo '    bin.install "mdview"'
    echo '  end'
    echo 'end'
} >"$out"

echo "已写 $out（v$ver）；brew audit --strict --online 想较真会问 bottle，这里是有意的：装的是 release 产物，不是 brew 自己编的包。"

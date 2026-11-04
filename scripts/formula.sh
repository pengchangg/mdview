#!/usr/bin/env bash
# 生成 Formula/mdview.rb：从 GitHub Release 下载各平台的 tarball，算 sha256，写进 brew formula。
# Homebrew 的 sha256 必须是**已发布** tarball 的哈希，所以这一步只能在 tag 之后跑：
# tag → CI 出 tarball → scripts/formula.sh <version> → 提交 Formula/mdview.rb（tag.sh 不碰这里）。
#
# 用法: scripts/formula.sh 0.4.0     # 写 Formula/mdview.rb，不改版本号以外的任何文件
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
        # -f：404 时不落盘，好区分「这个版本还没出这个平台」。代理照常走 https_proxy 环境变量。
        if ! hash="$(curl -fsSL --retry 3 --retry-delay 2 "$url" | sha256sum | cut -d' ' -f1)"; then
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
    echo "# 由 scripts/formula.sh $ver 生成，别手改：sha256 来自已发布的 tarball。"
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

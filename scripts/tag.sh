#!/usr/bin/env bash
# 打 tag 之前把版本号同步进 Cargo.toml（顺带刷新 Cargo.lock），一起提交后再打 tag 推出去。
# CI 的 publish 有一道「tag 版本必须等于 Cargo.toml」的硬校验，而 tag 指向的内容已经定死，
# 事后改不动（v0.3.0 就是这么挂的），所以同步只能发生在打 tag 之前 —— 这脚本就是为它加的。
#
# 用法: scripts/tag.sh 0.3.1     # = 改版本号 → 提交 → tag v0.3.1 → push 当前分支和 tag
set -euo pipefail

ver="${1:-}"
if ! [[ "$ver" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
    echo "用法: scripts/tag.sh <version>，例如 0.3.1（预发布可写 0.3.1-rc.1）" >&2
    exit 1
fi

cd "$(dirname "$0")/.."
tag="v$ver"

if git rev-parse -q --verify "refs/tags/$tag" >/dev/null; then
    echo "tag $tag 已经存在了；要重发就换个版本号，或先删掉旧 tag" >&2
    exit 1
fi
if [[ -n "$(git status --porcelain)" ]]; then
    echo "工作区不干净，先提交或 stash —— 版本号提交里不该夹带别的东西" >&2
    exit 1
fi

# 只改 [package] 下面第一个 version —— 后面每个依赖都有同名键，别误伤。
tmp="$(mktemp)"
awk -v v="$ver" 'done || !/^version = / { print; next }
                { print "version = \"" v "\"" ; done = 1 }' Cargo.toml >"$tmp"
mv "$tmp" Cargo.toml
# 只让 Cargo.lock 跟上新版本，不重解依赖。用 generate-lockfile 会把无关依赖一并升上
# 去（实测 pest_stack_guard 1.0.0 → 1.0.1），版本提交里不该夹带那种变动。
if ! cargo metadata --offline --format-version 1 >/dev/null; then
    git checkout -- Cargo.toml
    echo "刷新 Cargo.lock 失败（依赖没缓存就先 cargo fetch），Cargo.toml 已还原" >&2
    exit 1
fi

branch="$(git rev-parse --abbrev-ref HEAD)"
if ! git diff --quiet; then              # 版本号本来就对时别做空提交
    git add Cargo.toml Cargo.lock
    git commit -m "chore: bump version to $ver"
fi
# 先推分支再打 tag：一次 push 两个 ref 时，分支被拒而 tag 推上去的话，
# CI 就会从一个没进主干的提交上发版。
git push origin "$branch" || { echo "$branch 没推上去，tag 就不打了（版本号提交还在本地）" >&2; exit 1; }
git tag "$tag"
git push origin "$tag" || { echo "tag 没推上去；重试前先 git tag -d $tag" >&2; exit 1; }
echo "已推 $branch + $tag，CI 会跑 check → build ×3 → release → publish"

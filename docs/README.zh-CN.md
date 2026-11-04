# mdview

[English](../README.md) | [中文](README.zh-CN.md)

终端里的 Markdown 预览器。打开一个 `.md`，就能滚动、搜索、看大纲、看 mermaid 图，
**编辑器保存后画面立刻跟着变**。跨平台（Linux / macOS / Windows），纯 Rust，不需要 Node、浏览器或任何外部服务。

![mdview 在终端里滚动、搜索、开大纲](../examples/demo.gif)

```
mdview README.md
```

## 特性

- **全终端 TUI**：进入备用屏，退出后原样还你一个 shell。
- **完整 CommonMark + GFM**：标题、列表、任务列表、表格、删除线、脚注、Math、front matter、代码高亮（渲染交给 [`tui-markdown`](https://crates.io/crates/tui-markdown)）。
- **CJK 友好**：按显示宽度折行，中文不会把表格和代码框画歪。
- **mermaid 出图**：`graph` / `flowchart` / `sequenceDiagram` / `pie` / `classDiagram` / `gantt` … 直接画成字符图形（[`mermaid-text`](https://crates.io/crates/mermaid-text)），压不进列宽就自动逐级压缩；画不出来就原样贴回代码块并在状态栏提示，不会整屏崩。
- **热更新**：监听文件所在目录（编辑器的 rename-swap 也接得住），静默 250ms 后重绘。
- **同目录翻页**：可以直接指向一个目录（或者什么都不指），`]` / `[` 在这份文件的同目录 `.md` 之间走（按自然序，`part2` 排在 `part10` 前面）。
- **跟随链接**：`gx` 跳到 `#标题`、打开 `other.md`（`other.md#标题` 也行）、或者换进被链到的目录。外链只在状态栏显示，不会替你去开。
- **搜索**：`/` 接正则（普通文本也照旧能用），边打字边高亮，`n` / `N` 上下个匹配环绕，状态栏显示 `3/17`。被软拆成几行的句子仍然当一个整体去搜，也只亮它真的占到的那几行。
- **大纲侧栏**：`o` 开关，选中项跟着阅读位置走，`Enter` 跳过去。
- **窄栏阅读**：`--max-width 90` 限制正文列数，长行读着不累。
- **主题**：深/浅两套；`NO_COLOR`、`TERM=dumb` 自动降级为无色/ASCII。
- **配置文件 + 补全**：六个 TOML 键（`max_width`、`theme`、`mermaid`、`outline`、`no_watch`、`no_mouse`），命令行仍然压过它；`--print-completion bash|zsh|fish` 直接吐补全脚本。
- **不弄脏终端**：正常退出、`Ctrl-C`、甚至 panic 都会还原 raw mode、备用屏和鼠标捕获。

## 安装

```sh
# 一行命令，不需要 cargo（Linux x86_64/aarch64、Apple Silicon macOS、Windows x86_64）
curl -fsSL https://raw.githubusercontent.com/pengchangg/mdview/main/install.sh | sh
#   MDVIEW_PREFIX=~/.local/bin MDVIEW_VERSION=0.4.0 sh install.sh   # 装到哪、装哪个版本

brew tap pengchangg/mdview && brew install mdview   # 同一批包，Formula/ 里钉了 sha256

cargo install mdview-rs       # 从 crates.io 安装（装上的命令仍然是 mdview）
cargo install --path .        # 从当前仓库
cargo build --release         # 二进制在 target/release/mdview
```

`install.sh` 要 glibc（Alpine/musl 请用 `cargo install`），也没有 Intel macOS 的包 —— 那个环同样走 cargo。

crates.io 上的包名带 `-rs` 只是因为 `mdview` 这个名字 2020 年就被别人占用了；库名和命令都还是 `mdview`。

发版只用一条命令：`scripts/tag.sh 0.4.0`。它会把版本号同步进 `Cargo.toml` 和 `Cargo.lock`，提交、
推分支，再打 tag 推上去；随后 GitHub Actions 从那个 tag 发出四个压缩包（Linux x86_64 / aarch64、
macOS arm64、Windows x86_64），并发到 crates.io。解压后把单个 `mdview` 放进 `PATH` 即可。
同一条流水线随后会用刚发出的包算 sha256，重写 `Formula/mdview.rb` 并提交回 `main`，brew tap 不会
漏在一个旧版本上；`scripts/formula.sh <version>` 是这一步的手动版。

版本号必须在 tag 里面就是对的：CI 的二进制是从 tag 那份代码构建的，而 publish 前的检查只会比对
tag 和 `Cargo.toml`，不会（也没法）事后改 tag。v0.3.0 就是打在 `Cargo.toml` 还是 0.2.0 的提交上，
结果那个 release 里的二进制 `--version` 报 0.2.0，crates.io 也没收到东西。

## 用法

```sh
mdview README.md                 # 打开预览
mdview docs/                     # 打开目录里第一个 .md
mdview                           # ……或者当前目录里的第一个 .md
mdview docs/notes.md --outline   # 启动就展开大纲
mdview book.md --max-width 90    # 窄栏阅读
mdview big.md --no-watch         # 不监听文件
mdview chart.md --mermaid off    # mermaid 当代码块显示
mdview x.md --theme light        # 浅色终端
mdview x.md --no-mouse           # 启动就关掉鼠标捕获
```

## 按键

| 键 | 作用 |
| --- | --- |
| `j` `k` `↑` `↓` | 上下滚一行 |
| `d` `u` | 半页 |
| `f` `b` `PgDn` `PgUp` | 整页 |
| `gg` `G` `Home` `End` | 首行 / 末行（`g` 是前缀键，跟 vim 一样） |
| `]` `[` | 同目录下下一份 / 上一份 `.md`，到头环绕 |
| `gx` | 跟随光标往下最近的一条链接：`#锚点` → 跳转，`a.md` / `a.md#h` → 打开它，`dir/` → 换进那个目录，URL → 只显示 |
| `/` | 正则搜索，默认忽略大小写（结尾加 `\C` 区分，`\c` 再关回去）；`Enter` 确认，`Backspace` 退格，`Ctrl-U` 清空，`Esc` 取消 |
| `n` `N` | 下一个 / 上一个匹配，到末尾环绕 |
| `o` | 开关大纲侧栏（打开时焦点自动进去） |
| `Tab` | 焦点在正文 ↔ 大纲之间切换 |
| `j` `k` `g` `G` `Enter` | 大纲里：移动 / 跳到那个标题 |
| `Esc` | 大纲 → 正文；没有搜索词时 → 退出 |
| 鼠标滚轮 | 滚动当前焦点区 |
| 鼠标左键 | 点大纲条目 = 选中并跳转 |
| `m` | 切换鼠标捕获。关掉之后就能用终端自带的选中复制 |
| `?` | 帮助浮层（按任意键关闭） |
| `q` `Esc` `Ctrl-C` | 退出 |

程序内的提示文案（`--help`、状态栏、`?` 浮层、报错）一律英文；这份文档保持中文。

## 渲染约定

- 标题不显示 `#`（预览不是源码编辑器），靠颜色和加粗分层级；代码块也不贴 ``` 围栏。都是 `theme.rs` 里 `heading_marker` / `code_block_fence` 返回空串关掉的。
- 大纲认标题靠「整行 style == `heading(level)`」+ 文本前缀，所以代码块里以 `#` 开头的注释行不会被当成标题。

## 配置

```toml
# ~/.config/mdview/config.toml
max_width = 90
theme = "dark"        # auto | dark | light
mermaid = "auto"      # auto | off
outline = true
no_watch = false
no_mouse = false
```

查找顺序：`$MDVIEW_CONFIG`（一个文件路径，必须存在）→ `$XDG_CONFIG_HOME/mdview/config.toml` →
`~/.config/mdview/config.toml`。命令行压过配置文件，配置文件压过内置默认。里面写错了 —— 不认识的
键、类型不对、TOML 语法崩 —— 会直接启动失败并把读的是哪个路径打出来，而不是默默忽略。`max_width`
小于 20 会被夹到 20，并在状态栏提示一句。

## 已知限制（v0.4）

- 链接靠键盘（`gx`）跟，不靠鼠标：没做 span 级别的命中测试，外链也不会替你去开（只在状态栏显示 —— 终端支持的话仍然可以 `Ctrl+点击` OSC 8 链接）。链接是从源码里读的，所以写法很冷（`<a b.md>`、`%20` 转义）可能被当成“找不到”。没有文件树、多文件标签。
- 正则语法是 [`regex`](https://docs.rs/regex) crate 那一套 —— `a|b`、`(ab)+`、`\b`、`(?i)` 直接写，不是 vim 那套（元字符不用反斜杠反着加，`\v`/`\V` 也没意义，要找字面的 `$` 就写 `\$`）。编译成线性时间自动机，不会有灾难性回溯，但不支持反向引用和前瞻（`\1`、`(?=…)`）。
- 跳过硬换行的命中，只亮它占到的那几行，不整段亮；零宽匹配（`a*`）画不出高亮，直接丢弃。写到一半的正则（未闭合的 `(`）在状态栏显示错因，不弹窗。
- 没有模糊匹配；搜索框只有退格和 `Ctrl-U`，不支持左右移光标。
- mermaid 只出字符图，不出真图：不用 `mmdc`/Node/Chromium，也不依赖图像协议（Alacritty、Windows Terminal 都不支持 Kitty/ichthys）。
- 监听是按目录的（接编辑器 rename-swap 就靠这个）；配置文件只有六个键，没有 `[theme.dark]` 这种分段 —— 调色板只能选 `dark` / `light`，不能自己改。
- 鼠标开启时终端无法选中复制 —— 按 `m` 关掉即可。
- mermaid 字符图的上游小瑕疵（[`mermaid-text`](https://crates.io/crates/mermaid-text) 自身）：`gitGraph` 合并分支后的 commit 位置会偏、`classDiagram` 的 `«interface»` 与框线偶尔重叠一格 —— 图依然可读，等上游修。

## 开发

```sh
cargo test                                              # 单元测试 + 快照 + 逻辑测试（不起终端）
INSTA_UPDATE=always cargo test --test render_snapshot   # 改了渲染后更新快照
cargo run -- examples/demo.md
cargo clippy --all-targets -- -D warnings
vhs examples/demo.tape                                  # 录演示 gif（要 vhs + ttyd/tmux/ffmpeg + 一个中日韩字体）
```

代码分成两半：`render`/`wrap`/`search`/`mermaid`/`files`/`config`/`app` 是纯逻辑（终端 IO 一行都没有，所以全部可以单测），
`view`/`term`/`event`/`main` 才碰屏幕。渲染只有一个入口 `mdview::to_doc(src, width, opts)`，
宽度变了才重渲染，只变高度就只夹住滚动位置。

实测（release，1 MB 中英混排 + 大量代码块）：整篇渲染 369 ms、100 列 56609 行；搜索每按一个键全量重扫 6.8 ms。

## License

MIT

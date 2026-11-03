# mdview

终端里的 Markdown 预览器。打开一个 `.md`，就能滚动、搜索、看大纲、看 mermaid 图，
**编辑器保存后画面立刻跟着变**。跨平台（Linux / macOS / Windows），纯 Rust，不需要 Node、浏览器或任何外部服务。

```
mdview README.md
```

## 特性

- **全终端 TUI**：进入备用屏，退出后原样还你一个 shell。
- **完整 CommonMark + GFM**：标题、列表、任务列表、表格、删除线、脚注、Math、front matter、代码高亮（渲染交给 [`tui-markdown`](https://crates.io/crates/tui-markdown)）。
- **CJK 友好**：按显示宽度折行，中文不会把表格和代码框画歪。
- **mermaid 出图**：`graph` / `flowchart` / `sequenceDiagram` / `pie` / `classDiagram` / `gantt` … 直接画成字符图形（[`mermaid-text`](https://crates.io/crates/mermaid-text)），压不进列宽就自动逐级压缩；画不出来就原样贴回代码块并在状态栏提示，不会整屏崩。
- **热更新**：监听文件所在目录（编辑器的 rename-swap 也接得住），静默 250ms 后重绘。
- **搜索**：`/` 边打字边高亮，`n` / `N` 上下个匹配环绕，状态栏显示 `3/17`。
- **大纲侧栏**：`o` 开关，选中项跟着阅读位置走，`Enter` 跳过去。
- **窄栏阅读**：`--max-width 90` 限制正文列数，长行读着不累。
- **主题**：深/浅两套；`NO_COLOR`、`TERM=dumb` 自动降级为无色/ASCII。
- **不弄脏终端**：正常退出、`Ctrl-C`、甚至 panic 都会还原 raw mode、备用屏和鼠标捕获。

## 安装

```sh
cargo install mdview
```

或从 [Releases](../../releases) 下载三平台的压缩包，解压后把单个 `mdview` 放进 `PATH`。

```sh
# 从源码
git clone https://github.com/your-name/mdView && cd mdView && cargo build --release
```

## 用法

```sh
mdview README.md                 # 打开预览
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
| `g` `G` `Home` `End` | 首行 / 末行 |
| `/` | 搜索（大小写不敏感子串），`Enter` 确认，`Backspace` 退格，`Ctrl-U` 清空，`Esc` 取消 |
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

## 渲染约定

- 标题不显示 `#`（预览不是源码编辑器），靠颜色和加粗分层级；代码块也不贴 ``` 围栏。都是 `theme.rs` 里 `heading_marker` / `code_block_fence` 返回空串关掉的。
- 大纲认标题靠「整行 style == `heading(level)`」+ 文本前缀，所以代码块里以 `#` 开头的注释行不会被当成标题。

## 已知限制（v0.1）

- 链接不做跳转（只带 OSC 8，终端支持的话可以 `Ctrl+点击`）；没有文件树、多文件标签。
- 搜索是子串匹配，不支持正则和模糊；搜索框只有退格和 `Ctrl-U`，不支持左右移光标。
- mermaid 只出字符图，不出真图：不用 `mmdc`/Node/Chromium，也不依赖图像协议（Alacritty、Windows Terminal 都不支持 Kitty/ichthys）。
- 不支持 `--watch` 之外的目录级监听，配置文件系统还没有（只有命令行参数）。
- 鼠标开启时终端无法选中复制 —— 按 `m` 关掉即可。
- mermaid 字符图的上游小瑕疏（[`mermaid-text`](https://crates.io/crates/mermaid-text) 自身）：`gitGraph` 合并分支后的 commit 位置会偏、`classDiagram` 的 `«interface»` 与框线偶尔重叠一格 —— 图依然可读，等上游修。

## 开发

```sh
cargo test              # 单元测试 + 快照 + 逻辑测试（不起终端）
INSTA_UPDATE=always cargo test --test render_snapshot   # 改了渲染后更新快照
cargo run -- examples/demo.md
cargo clippy --all-targets -- -D warnings
```

代码分成两半：`render`/`wrap`/`search`/`mermaid`/`app` 是纯逻辑（终端 IO 一行都没有，所以全部可以单测），
`view`/`term`/`event`/`main` 才碰屏幕。渲染只有一个入口 `mdview::to_doc(src, width, opts)`，
宽度变了才重渲染，只变高度就只夹住滚动位置。

实测（release，1 MB 中英混排 + 大量代码块）：整篇渲染 369 ms、100 列 56609 行；搜索每按一个键全量重扫 6.8 ms。

## License

MIT

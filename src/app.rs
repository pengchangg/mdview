//! 应用状态与按键逻辑（不碰终端，纯逻辑，可单测）。

use std::path::{Path, PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

use crate::cli::{Args, MermaidMode};
use crate::event::Ev;
use crate::files;
use crate::render::{self, Heading, RenderOptions, RenderedDoc};
use crate::search::{self, Match};
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Text,
    Outline,
}

pub struct App {
    pub path: PathBuf,
    /// 同一目录里的 md 列表（自然序），`]` `[` 在里面走，状态栏拿它算 3/12。
    pub files: Vec<PathBuf>,
    /// 当前文件在 `files` 里的下标。
    pub file_idx: usize,
    pub args: Args,
    pub theme: Theme,
    pub src: String,
    pub doc: RenderedDoc,
    pub size: (u16, u16),
    pub scroll: usize,
    pub focus: Focus,
    pub query: String,
    pub editing: bool,
    pub matches: Vec<Match>,
    pub idx: usize,
    /// 上一次编译搜索词的错误（一行文案），非空表示 `query` 不是合法正则。
    pub search_err: Option<String>,
    pub outline_open: bool,
    pub sel: usize,
    pub help: bool,
    pub mouse: bool,
    /// 刚按下 `g`：vim 里 g 是前缀键，单按不动，下一个键是 x 就跟链接，是 g 就跳顶。
    pub pending_g: bool,
    pub quit: bool,
    pub status: String,
    pub body_rect: Rect,
    pub panel_rect: Rect,
    pre_search_scroll: usize,
}

impl App {
    pub fn open(args: Args, theme: Theme, size: (u16, u16)) -> color_eyre::Result<Self> {
        // 命令行给的是文件或目录（或不给）；展开成「一串文件 + 从哪个开始」再读第一个。
        let cwd = std::env::current_dir()?;
        let one: Vec<PathBuf> = args.file.iter().cloned().collect();
        let (files, file_idx) = files::resolve_targets(&one, &cwd)?;
        let path = files[file_idx].clone();
        let src = read_src(&path)?;
        let mut app = Self {
            doc: RenderedDoc::default(),
            src,
            path,
            files,
            file_idx,
            pre_search_scroll: 0,
            size,
            theme,
            scroll: 0,
            focus: Focus::Text,
            query: String::new(),
            editing: false,
            matches: Vec::new(),
            idx: 0,
            search_err: None,
            outline_open: args.outline.unwrap_or(false),
            sel: 0,
            help: false,
            mouse: !args.no_mouse.unwrap_or(false),
            pending_g: false,
            quit: false,
            status: String::new(),
            body_rect: Rect::default(),
            panel_rect: Rect::default(),
            args,
        };
        app.rerender();
        Ok(app)
    }

    // ---- 几何：宽度变了才重渲染，只有高度变就只 clamp ----

    /// 大纲面板宽度；未展开为 0。
    pub fn panel_width(&self) -> u16 {
        if !self.outline_open {
            return 0;
        }
        let w = self.size.0;
        if w < 48 {
            return 0; // 太窄就别占了
        }
        (w / 3).clamp(24, 40).min(w / 2)
    }

    /// 正文（含 mermaid、已按此宽度折行）可用列数，不含滚动条那一列。
    pub fn text_width(&self) -> u16 {
        let panel = self.panel_width();
        let gap = if panel > 0 { 1 } else { 0 };
        let left = self.size.0.saturating_sub(panel + gap);
        let mut w = left.saturating_sub(1); // 留一列滚动条
        if let Some(max) = self.args.max_width {
            w = w.min(max);
        }
        if w < 20 {
            w = left.max(1); // 极窄终端不留滚动条了
        }
        w
    }

    pub fn viewport(&self) -> usize {
        self.size.1.saturating_sub(1).max(1) as usize
    }

    pub fn max_scroll(&self) -> usize {
        self.doc.lines.len().saturating_sub(self.viewport())
    }

    pub fn rerender(&mut self) {
        let opts = RenderOptions {
            theme: self.theme,
            mermaid: self.args.mermaid.unwrap_or(MermaidMode::Auto),
        };
        self.doc = render::to_doc(&self.src, self.text_width(), opts);
        let q = self.query.clone();
        self.set_query(&q);
        self.clamp();
    }

    pub fn set_size(&mut self, w: u16, h: u16) {
        let before = self.text_width();
        self.size = (w, h);
        if self.text_width() != before {
            self.rerender();
        } else {
            self.clamp();
        }
    }

    /// 重读文件；内容真变了才重渲染，避免编辑器空写抖一下。
    pub fn reload(&mut self) {
        match read_src(&self.path) {
            Ok(s) if s != self.src => {
                self.src = s;
                self.rerender();
                // rerender 会清状态，重载提示要放在它后面。
                self.status = format!("reloaded {}", self.file_name());
            }
            Ok(_) => {}
            Err(e) => self.status = format!("read failed: {e}"),
        }
    }

    // ---- 多文件 ----

    /// 切到 `files[i]`：读得到才切，读失败只报状态、屏幕不动。
    /// 搜索态不带进下一个文件（匹配行号到新文件上没意义），滚动和大纲选择归零。
    /// 打开列表里的第 i 个文件；读不到就只报状态、屏幕不动，返回 false。
    pub fn open_file(&mut self, i: usize) -> bool {
        let Some(path) = self.files.get(i).cloned() else {
            return false;
        };
        match read_src(&path) {
            Ok(src) => {
                self.path = path;
                self.file_idx = i;
                self.src = src;
                self.query.clear();
                self.matches.clear();
                self.search_err = None;
                self.pre_search_scroll = 0;
                self.scroll = 0;
                self.idx = 0;
                self.sel = 0;
                self.rerender();
                true
            }
            Err(e) => {
                self.status = format!("read failed: {e}");
                false
            }
        }
    }

    /// `]` / `[`：兄弟文件之间循环。
    pub fn step_file(&mut self, delta: isize) {
        let n = self.files.len();
        if n < 2 {
            return;
        }
        let next = (self.file_idx as isize + delta).rem_euclid(n as isize) as usize;
        self.open_file(next);
    }

    /// 重新列一遍当前目录：外部增删文件都跟得上；当前文件被删也留在列表里，屏幕内容不动。
    pub fn refresh_listing(&mut self) {
        let Some(dir) = self.path.parent() else {
            return;
        };
        let mut listed = files::list_md(dir);
        self.file_idx = files::insert_sorted(&mut listed, self.path.clone());
        self.files = listed;
    }

    // ---- 跟随链接 (gx) ----

    /// 跟光标下方（含当前行）最近的一条链接：锚点跳标题，文件/目录换文件，URL 只报到状态栏。
    pub fn follow_link(&mut self) {
        let Some(link) = self.doc.links.iter().find(|l| l.line >= self.scroll) else {
            self.status = "no link at cursor".into();
            return;
        };
        let url = link.url.clone();
        self.open_target(&url);
    }

    fn open_target(&mut self, url: &str) {
        let url = url.trim();
        if url.is_empty() {
            self.status = "empty link".into();
            return;
        }
        if let Some(anchor) = url.strip_prefix('#') {
            return self.jump_to_anchor(anchor);
        }
        // 带协议的一律不拉外部程序：状态栏把 url 说出来，支持 ctrl-click 的终端自己点。
        // shortcut: javascript: 之类也走这条（只当文本），要真拦截再说。
        if url.contains("://") || url.starts_with("mailto:") {
            self.status = format!("{url} — ctrl-click opens it");
            return;
        }
        let (raw, anchor) = url.split_once('#').unwrap_or((url, ""));
        // shortcut: 不做 %XX 解码；手写的链接没有 %20，真撞上再加。
        let base = self
            .path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let joined = files::absolute(Path::new(raw), &base);
        if joined.is_dir() {
            let listed = files::list_md(&joined);
            if listed.is_empty() {
                self.status = format!("⚠ no .md file in {raw}");
                return;
            }
            self.files = listed;
            if anchor.is_empty() {
                self.open_file(0);
            } else if self.open_file(0) {
                self.jump_to_anchor(anchor);
            }
            return;
        }
        if !joined.is_file() {
            self.status = format!("⚠ not found: {raw}");
            return;
        }
        let mut listed = self.files.clone();
        let idx = files::insert_sorted(&mut listed, joined);
        self.files = listed;
        if anchor.is_empty() {
            self.open_file(idx);
        } else if self.open_file(idx) {
            self.jump_to_anchor(anchor);
        }
    }

    /// 按标题文本跳锚点（GitHub slug 口径），命中第一条；重名就在状态栏说有几条。
    pub fn jump_to_anchor(&mut self, anchor: &str) {
        let want = anchor_key(anchor);
        let hits: Vec<usize> = self
            .doc
            .headings
            .iter()
            .enumerate()
            .filter(|(_, h)| anchor_key(&h.text) == want)
            .map(|(i, _)| i)
            .collect();
        let Some(&first) = hits.first() else {
            self.status = format!("⚠ no heading \"{}\"", anchor.trim());
            return;
        };
        self.sel = first;
        self.focus = Focus::Text;
        self.jump_line(self.doc.headings[first].line);
        if hits.len() > 1 {
            self.status = format!(
                "{} headings named \"{}\", went to the first",
                hits.len(),
                anchor.trim()
            );
        }
    }

    fn clamp(&mut self) {
        let max = self.max_scroll();
        self.scroll = self.scroll.min(max);
        self.sel = self.sel.min(self.doc.headings.len().saturating_sub(1));
        if self.idx > 0 {
            self.idx = self.idx.min(self.matches.len().saturating_sub(1));
        }
    }

    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.path.display().to_string())
    }

    // ---- 滚动 ----

    pub fn scroll_by(&mut self, delta: isize) {
        let next = self.scroll as isize + delta;
        self.scroll = if next < 0 { 0 } else { next as usize }.min(self.max_scroll());
    }

    pub fn page(&mut self, dir: isize) {
        let step = self.viewport() as isize / 2;
        self.scroll_by(if dir > 0 { step } else { -step });
    }

    pub fn full_page(&mut self, dir: isize) {
        let step = self.viewport() as isize;
        self.scroll_by(if dir > 0 { step } else { -step });
    }

    pub fn goto_top(&mut self) {
        self.scroll = 0;
    }

    pub fn goto_bottom(&mut self) {
        self.scroll = self.max_scroll();
    }

    pub fn jump_line(&mut self, line: usize) {
        self.scroll = line.min(self.max_scroll());
    }

    pub fn line_of_heading(&self, i: usize) -> Option<usize> {
        self.doc.headings.get(i).map(|h| h.line)
    }

    // ---- 搜索 ----

    pub fn begin_search(&mut self) {
        self.editing = true;
        self.pre_search_scroll = self.scroll;
    }

    pub fn push_query(&mut self, c: char) {
        self.query.push(c);
        self.apply_query(true);
    }

    pub fn pop_query(&mut self) {
        self.query.pop();
        self.apply_query(true);
    }

    pub fn set_query(&mut self, q: &str) {
        self.query = q.to_string();
        self.apply_query(false);
    }

    fn apply_query(&mut self, typing: bool) {
        let pat = match search::compile(&self.query) {
            Ok(p) => p,
            Err(e) => {
                self.matches.clear();
                self.idx = 0;
                self.search_err = Some(e.clone());
                self.status = format!("invalid pattern: {e}");
                return;
            }
        };
        self.search_err = None;
        self.matches = search::find(&self.doc.logical, &pat);
        if self.matches.is_empty() {
            self.idx = 0;
            self.status = if self.query.is_empty() {
                String::new()
            } else {
                format!("no match for \"{}\"", self.query)
            };
            return;
        }
        // 打字时以进入搜索时的位置为锚点（越打越窄不会一路往下跑）；
        // 重渲染时以当前位置为准，命中还在视口里就不动。
        let from = if typing {
            self.pre_search_scroll
        } else {
            self.scroll
        };
        self.idx = self
            .matches
            .iter()
            .position(|m| m.line >= from)
            .unwrap_or(0);
        let line = self.matches[self.idx].line;
        if typing || line < self.scroll || line >= self.scroll + self.viewport() {
            self.jump_to(line);
        }
    }

    pub fn end_search(&mut self) {
        self.editing = false;
        if self.matches.is_empty() && !self.query.is_empty() {
            self.status = format!("no match for \"{}\"", self.query);
        }
    }

    pub fn cancel_search(&mut self) {
        self.editing = false;
        if self.query.is_empty() {
            return;
        }
        self.query.clear();
        self.matches.clear();
        self.search_err = None;
        self.scroll = self.pre_search_scroll.min(self.max_scroll());
    }

    /// 环绕跳到下一个（back=true 上一个）匹配。
    pub fn step_match(&mut self, back: bool) {
        // 正则没编译对时 matches 必定为空，状态栏里已是编译错误，别覆成「没找到」。
        if self.search_err.is_some() {
            return;
        }
        if self.matches.is_empty() {
            self.status = if self.query.is_empty() {
                "no search query".into()
            } else {
                format!("no match for \"{}\"", self.query)
            };
            return;
        }
        let n = self.matches.len();
        self.idx = if back {
            (self.idx + n - 1) % n
        } else {
            (self.idx + 1) % n
        };
        self.jump_to(self.matches[self.idx].line);
    }

    fn jump_to(&mut self, line: usize) {
        // 命中行至少露在视口上半部分，别让它在最底下一行贴着状态栏。
        let target = line.saturating_sub(self.viewport() / 3);
        self.scroll = target.min(self.max_scroll());
    }

    // ---- 大纲 ----

    pub fn toggle_outline(&mut self, focus_it: bool) {
        self.outline_open = !self.outline_open;
        if self.outline_open {
            self.sel = self.nearest_heading();
            if focus_it {
                self.focus = Focus::Outline;
            }
            self.rerender(); // 正文宽度变了
        } else {
            self.focus = Focus::Text;
            self.rerender();
        }
    }

    /// 当前视口顶行往上最近的标题 —— 打开大纲时把选中项对齐到读到哪儿了。
    pub fn nearest_heading(&self) -> usize {
        self.doc
            .headings
            .iter()
            .rposition(|h| h.line <= self.scroll)
            .unwrap_or_default()
    }

    pub fn outline_step(&mut self, delta: isize) {
        if self.doc.headings.is_empty() {
            return;
        }
        let len = self.doc.headings.len() as isize;
        let s = (self.sel as isize + delta).clamp(0, len - 1);
        self.sel = s as usize;
    }

    pub fn outline_jump(&mut self) {
        if let Some(h) = self.doc.headings.get(self.sel) {
            let line = h.line;
            self.jump_to(line);
        }
    }

    // ---- 事件 ----

    pub fn handle(&mut self, evs: Vec<Ev>) {
        for ev in evs {
            match ev {
                Ev::Key(k) => self.on_key(k),
                Ev::Mouse(m) => self.on_mouse(m),
                Ev::Resize(w, h) => self.set_size(w, h),
                Ev::FileChanged => {
                    self.refresh_listing();
                    self.reload();
                }
                Ev::Focus(_) => {}
            }
        }
    }

    pub fn on_key(&mut self, k: KeyEvent) {
        if k.kind == KeyEventKind::Release {
            return;
        }
        if k.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(k.code, KeyCode::Char('c') | KeyCode::Char('d'))
        {
            self.quit = true;
            return;
        }

        if self.editing {
            if k.modifiers.contains(KeyModifiers::CONTROL) {
                // Ctrl-U 清空搜索框；其余 Ctrl-* 不吃，免得把控制字符当普通字符塞进词里。
                if matches!(k.code, KeyCode::Char('u')) {
                    self.query.clear();
                    self.apply_query(true);
                }
                return;
            }
            match k.code {
                KeyCode::Esc => self.cancel_search(),
                KeyCode::Enter => {
                    self.end_search();
                    if !self.matches.is_empty() {
                        self.step_match(k.modifiers.contains(KeyModifiers::SHIFT));
                    }
                }
                KeyCode::Backspace => self.pop_query(),
                KeyCode::Tab => self.step_match(k.modifiers.contains(KeyModifiers::SHIFT)),
                KeyCode::Char(c) => self.push_query(c),
                _ => {}
            }
            return;
        }

        if self.help {
            self.help = false;
            return;
        }

        // 大纲焦点：只吃导航键，其余键当「回到正文」再走一遍。
        if self.focus == Focus::Outline {
            match k.code {
                KeyCode::Char('j') | KeyCode::Down => return self.outline_step(1),
                KeyCode::Char('k') | KeyCode::Up => return self.outline_step(-1),
                KeyCode::Char('g') | KeyCode::Home => {
                    self.sel = 0;
                    return;
                }
                KeyCode::Char('G') | KeyCode::End => {
                    self.sel = self.doc.headings.len().saturating_sub(1);
                    return;
                }
                KeyCode::Enter => {
                    self.outline_jump();
                    return;
                }
                KeyCode::Esc | KeyCode::Tab | KeyCode::Char('o') => {
                    self.focus = Focus::Text;
                    return;
                }
                _ => self.focus = Focus::Text,
            }
        }

        // g 是前缀键：单按不动，下一个键才决定跟链接还是跳顶。
        if self.pending_g {
            self.pending_g = false;
            if k.code == KeyCode::Char('x') {
                self.follow_link();
                return;
            }
            self.goto_top();
            // gg 到此为止，别把第二个 g 再当下一个前缀（不然 gg 后按 x 会误跟链接）。
            if k.code == KeyCode::Char('g') {
                return;
            }
        }

        match k.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Esc => {
                if !self.query.is_empty() {
                    self.cancel_search();
                } else {
                    self.quit = true;
                }
            }
            KeyCode::Char('j') | KeyCode::Down => self.scroll_by(1),
            KeyCode::Char(']') => self.step_file(1),
            KeyCode::Char('[') => self.step_file(-1),
            KeyCode::Char('k') | KeyCode::Up => self.scroll_by(-1),
            KeyCode::Char('d') => self.page(1),
            KeyCode::Char('u') => self.page(-1),
            KeyCode::Char('f') | KeyCode::PageDown => self.full_page(1),
            KeyCode::Char('b') | KeyCode::PageUp => self.full_page(-1),
            KeyCode::Char('g') => self.pending_g = true,
            KeyCode::Home => self.goto_top(),
            KeyCode::Char('G') | KeyCode::End => self.goto_bottom(),
            KeyCode::Char('o') => self.toggle_outline(true),
            KeyCode::Char('/') => self.begin_search(),
            KeyCode::Char('?') => self.help = true,
            KeyCode::Char('n') => self.step_match(false),
            KeyCode::Char('N') => self.step_match(true),
            KeyCode::Char('m') => {
                self.mouse = !self.mouse;
                crate::term::set_mouse(self.mouse);
                self.status = if self.mouse {
                    "mouse capture: on"
                } else {
                    "mouse capture: off (terminal selection works again)"
                }
                .into();
            }
            KeyCode::Tab if self.outline_open => {
                self.focus = Focus::Outline;
                self.sel = self.nearest_heading();
            }
            _ => {}
        }
    }

    pub fn on_mouse(&mut self, m: MouseEvent) {
        let on_panel = self.panel_rect != Rect::default()
            && m.column >= self.panel_rect.x
            && m.column < self.panel_rect.x + self.panel_rect.width
            && m.row >= self.panel_rect.y
            && m.row < self.panel_rect.y + self.panel_rect.height;
        match m.kind {
            MouseEventKind::ScrollUp => {
                if self.focus == Focus::Outline {
                    self.outline_step(-3)
                } else {
                    self.scroll_by(-3)
                }
            }
            MouseEventKind::ScrollDown => {
                if self.focus == Focus::Outline {
                    self.outline_step(3)
                } else {
                    self.scroll_by(3)
                }
            }
            MouseEventKind::Down(crossterm::event::MouseButton::Left) if on_panel => {
                self.focus = Focus::Outline;
                let inner = self.panel_rect.y + 1; // 边框占一行
                self.sel = ((m.row.saturating_sub(inner)) as usize)
                    .min(self.doc.headings.len().saturating_sub(1));
                self.outline_jump();
            }
            _ => {}
        }
    }

    /// 当前匹配在正文里的行号（给滚动条/状态栏用）。
    pub fn current_match_line(&self) -> Option<usize> {
        self.matches.get(self.idx).map(|m| m.line)
    }

    pub fn headings(&self) -> &[Heading] {
        &self.doc.headings
    }
}

/// 读源文件。BOM 得剔掉，不然首行标题前面挂个空字符，大纲第一项也对不上。
/// 锚点比对走 GitHub slug：小写、非字母数字都当分隔符，所以 `#getting-started` 能对上「Getting Started」。
fn anchor_key(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

fn read_src(path: &Path) -> color_eyre::Result<String> {
    let s = std::fs::read_to_string(path)?;
    Ok(s.strip_prefix('\u{feff}').unwrap_or(&s).to_string())
}

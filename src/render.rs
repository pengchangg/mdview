//! markdown → 可见行。产出搜索与大纲要用的三件东西：`lines` / `plain` / `headings`。

use ratatui::text::{Line, Span};
use tui_markdown::StyleSheet;
use unicode_width::UnicodeWidthStr;

use crate::cli::MermaidMode;
use crate::mermaid::{self, Seg};
use crate::theme::{Sheet, Theme};
use crate::wrap::{truncate, wrap_line};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    /// 在 `RenderedDoc::lines` 里的行号。
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub text: String,
    pub url: String,
    /// 链接在 `RenderedDoc::lines` 里的起始显示行（gx 拿它找「光标下面那一条」）。
    pub line: usize,
}

/// 一条「逻辑行」= 折行之前的整段文本 + 它折出的每个显示行在其中的字节区间。
/// 搜索在 `text` 上匹配（软换行的段落连着搜得到），画的时候再按 `spans` 摊回显示行。
#[derive(Debug, Clone, Default)]
pub struct Logical {
    pub text: String,
    /// 第一个显示行在 `lines` 里的行号（其余行紧跟着）。
    pub first_row: usize,
    /// 与显示行一一对应；区间是这一行的文本在 `text` 里的字节范围。
    pub spans: Vec<(usize, usize)>,
}

#[derive(Debug, Default)]
pub struct RenderedDoc {
    // shortcut: 整篇渲染结果全量常驻；实测 1 MB markdown 渲染出 56609 行能接受，
    // 再大就改成按 viewport 分块懒渲染。
    pub lines: Vec<Line<'static>>,
    /// 与 `lines` 一一对应的无样式文本。
    pub plain: Vec<String>,
    /// 未折行的逻辑行，按文档顺序；搜索索引（`plain` 是折完的显示行）。
    pub logical: Vec<Logical>,
    pub headings: Vec<Heading>,
    /// 文中链接，按出现顺序；gx 跟它们。
    pub links: Vec<Link>,
    /// 渲染期告警（如某个 mermaid 图没画出来），状态栏显示。
    pub warnings: Vec<String>,
}

impl RenderedDoc {
    pub fn plain_of(&self, i: usize) -> &str {
        self.plain.get(i).map(String::as_str).unwrap_or("")
    }
}

#[derive(Clone, Copy)]
pub struct RenderOptions {
    pub theme: Theme,
    pub mermaid: MermaidMode,
}

/// 唯一渲染入口。`width` 为正文可用列数。
/// 实测 1 MB markdown（含大量代码块高亮）100 列下 369 ms，只在打开/重渲染时跑。
pub fn to_doc(src: &str, width: u16, opts: RenderOptions) -> RenderedDoc {
    let w = width as usize;
    let mut doc = RenderedDoc::default();
    let segs = match opts.mermaid {
        MermaidMode::Off => vec![Seg::Md(src.to_string())],
        MermaidMode::Auto => mermaid::split(src),
    };

    // shortcut: 重渲染时每张图都重画（单张几毫秒）；等到图多到热更新能感觉到卡，
    // 再按 (图源码, 宽度) 做记忆化。
    for seg in segs {
        match seg {
            Seg::Md(md) => push_md(&mut doc, &md, w, opts),
            Seg::Diagram(code) => match mermaid::render(&code, w, opts.theme.ascii) {
                Some(pic) => {
                    push_diagram(&mut doc, &pic, &mermaid::kind_label(&code), w, opts.theme)
                }
                None => {
                    doc.warnings.push(format!(
                        "mermaid diagram failed to render ({})",
                        mermaid::kind_label(&code)
                    ));
                    push_md(&mut doc, &fenced(&code), w, opts);
                }
            },
        }
    }
    doc.plain = doc.lines.iter().map(plain_of).collect();
    doc
}

fn fenced(code: &str) -> String {
    format!("```mermaid\n{code}```\n")
}

/// 追加一条「自己就是一行」的行（图表边框、空行）：它自己算一个逻辑行。
fn push_one(doc: &mut RenderedDoc, line: Line<'static>) {
    let text = plain_of(&line);
    let len = text.len();
    doc.logical.push(Logical {
        text,
        first_row: doc.lines.len(),
        spans: vec![(0, len)],
    });
    doc.lines.push(line);
}

fn plain_of(line: &Line<'_>) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

fn push_md(doc: &mut RenderedDoc, md: &str, w: usize, opts: RenderOptions) {
    let sheet = Sheet(opts.theme);
    let options = tui_markdown::Options::new(sheet).table_width(w as u16);
    let text = tui_markdown::from_str_with_options(md, &options);
    let marks = source_marks(md);
    let mut next = 0usize;
    let mut next_link = 0usize;

    for line in &text.lines {
        if let Some(h) = marks.heads.get(next)
            && is_heading_line(line, h, sheet)
        {
            doc.headings.push(Heading {
                level: h.0,
                text: h.1.clone(),
                line: doc.lines.len(),
            });
            next += 1;
        }
        // 链接按「这一行的文本里出现下一条链接的文字」落行：同一行多条就都记下。
        // 文字在段首重复出现时可能把行号往前挪一行，但 url 是对的 —— gx 要的是 url。
        let flat = plain_of(line);
        while let Some(lk) = marks.links.get(next_link) {
            if flat.contains(lk.0.as_str()) {
                doc.links.push(Link {
                    text: lk.0.clone(),
                    url: lk.1.clone(),
                    line: doc.lines.len(),
                });
                next_link += 1;
            } else {
                break;
            }
        }
        let rows = wrap_line(line, w);
        let first_row = doc.lines.len();
        let spans = row_spans(&flat, &rows);
        doc.logical.push(Logical {
            text: flat,
            first_row,
            spans,
        });
        doc.lines.extend(rows);
    }
}

/// 折出来的每一行在逻辑文本里的字节区间。折行会丢掉断点处的空格（tab 还会展成
/// 4 空格），所以比对时两边都跳空白。
/// shortcut: 对不上就退化成「整段算第一行」，跨断点的命中只亮首行；tab 混进长段落才可能走到。
fn row_spans(text: &str, rows: &[Line<'static>]) -> Vec<(usize, usize)> {
    if rows.len() == 1 {
        return vec![(0, text.len())];
    }
    let cs: Vec<char> = text.chars().collect();
    let mut offs: Vec<usize> = Vec::with_capacity(cs.len() + 1);
    let mut o = 0usize;
    for c in &cs {
        offs.push(o);
        o += c.len_utf8();
    }
    offs.push(o);

    let mut out = Vec::with_capacity(rows.len());
    let mut cur = 0usize;
    for row in rows {
        while cur < cs.len() && cs[cur].is_whitespace() {
            cur += 1;
        }
        let start = offs[cur.min(cs.len())];
        let mut end = start;
        for c in plain_of(row).chars() {
            while cur < cs.len() && cs[cur] != c {
                if cs[cur].is_whitespace() {
                    cur += 1;
                } else {
                    return vec![(0, text.len())];
                }
            }
            if cur >= cs.len() {
                return vec![(0, text.len())];
            }
            cur += 1;
            end = offs[cur];
        }
        out.push((start, end));
    }
    out
}

fn push_diagram(doc: &mut RenderedDoc, pic: &str, kind: &str, w: usize, theme: Theme) {
    if doc.lines.last().is_some_and(|l| plain_of(l).trim() != "") {
        push_one(doc, Line::from(""));
    }
    let frame = theme.frame();
    let title = format!("── mermaid: {kind} ");
    push_one(
        doc,
        Line::from(vec![
            Span::styled(title.clone(), frame),
            Span::styled("─".repeat(w.saturating_sub(title.width()).max(1)), frame),
        ]),
    );
    for l in pic.lines() {
        let line = Line::styled(l.to_string(), frame);
        let rows = wrap_line(&line, w);
        let text = l.to_string();
        doc.logical.push(Logical {
            text: text.clone(),
            first_row: doc.lines.len(),
            spans: row_spans(&text, &rows),
        });
        doc.lines.extend(rows);
    }
    push_one(doc, Line::from(""));
}

/// 一趟 pulldown-cmark 遍历同时取标题和链接（与 tui-markdown 同一套解析选项）。
fn source_marks(md: &str) -> Marks {
    use pulldown_cmark::{Options as PdOptions, Parser, Tag};
    let mut p = PdOptions::empty();
    p.insert(PdOptions::ENABLE_GFM);
    p.insert(PdOptions::ENABLE_TABLES);
    p.insert(PdOptions::ENABLE_FOOTNOTES);
    p.insert(PdOptions::ENABLE_TASKLISTS);
    p.insert(PdOptions::ENABLE_MATH);
    p.insert(PdOptions::ENABLE_YAML_STYLE_METADATA_BLOCKS);
    p.insert(PdOptions::ENABLE_HEADING_ATTRIBUTES);

    let mut out = Marks::default();
    let mut depth = 0u8;
    let mut cur = String::new();
    let mut link: Option<(String, String)> = None;
    for ev in Parser::new_ext(md, p) {
        match ev {
            pulldown_cmark::Event::Start(Tag::Heading { level, .. }) => {
                depth = level as u8;
                cur.clear();
            }
            pulldown_cmark::Event::End(pulldown_cmark::TagEnd::Heading(_)) => {
                out.heads
                    .push((depth, std::mem::take(&mut cur).trim().to_string()));
                depth = 0;
            }
            // 图片不算链接：gx 不该去“打开”一张图。
            pulldown_cmark::Event::Start(Tag::Link { dest_url, .. }) => {
                link = Some((String::new(), dest_url.to_string()));
            }
            pulldown_cmark::Event::End(pulldown_cmark::TagEnd::Link) => {
                if let Some((text, url)) = link {
                    let text = text.trim().to_string();
                    if !text.is_empty() {
                        out.links.push((text, url));
                    }
                }
                link = None;
            }
            pulldown_cmark::Event::Text(t) => {
                if depth > 0 {
                    cur.push_str(&t);
                }
                if let Some((text, _)) = link.as_mut() {
                    text.push_str(&t);
                }
            }
            pulldown_cmark::Event::Code(c) if depth > 0 => cur.push_str(&c),
            pulldown_cmark::Event::SoftBreak if depth > 0 => cur.push(' '),
            _ => {}
        }
    }
    out
}

#[derive(Default)]
struct Marks {
    heads: Vec<(u8, String)>,
    links: Vec<(String, String)>,
}

/// 标题行靠「整行 style == heading(level)」+「文本前缀 == 期望标题」两个条件识别；
/// 不看 `#` 是因为样式表把 marker 藏了，也不能误认代码块里以 `#` 开头的注释行。
fn is_heading_line(line: &Line<'_>, want: &(u8, String), sheet: Sheet) -> bool {
    if line.style != sheet.heading(want.0) {
        return false;
    }
    let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
    let want_text = want.1.trim();
    want_text.is_empty() || text.trim_start().starts_with(want_text)
}

/// 大纲面板行：`  ▸ 标题`，按相对层级缩进。
pub fn outline_lines(
    headings: &[Heading],
    width: usize,
    selected: usize,
    theme: Theme,
) -> Vec<Line<'static>> {
    let min_level = headings.iter().map(|h| h.level).min().unwrap_or(1);
    headings
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let indent = "  ".repeat((h.level - min_level).min(4) as usize);
            let label = truncate(
                &format!(
                    "{}{indent}{}",
                    if i == selected { "▸ " } else { "  " },
                    h.text
                ),
                width,
            );
            let style = if i == selected {
                theme.outline_selected()
            } else {
                theme.text()
            };
            Line::styled(label, style)
        })
        .collect()
}

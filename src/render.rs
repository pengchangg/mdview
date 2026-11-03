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

#[derive(Debug, Default)]
pub struct RenderedDoc {
    // shortcut: 整篇渲染结果全量常驻；实测 1 MB markdown 渲染出 56609 行能接受，
    // 再大就改成按 viewport 分块懒渲染。
    pub lines: Vec<Line<'static>>,
    /// 与 `lines` 一一对应的无样式文本，搜索索引。
    pub plain: Vec<String>,
    pub headings: Vec<Heading>,
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
                        "mermaid 图未能渲染（{}）",
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

fn plain_of(line: &Line<'_>) -> String {
    line.spans.iter().map(|s| s.content.as_ref()).collect()
}

fn push_md(doc: &mut RenderedDoc, md: &str, w: usize, opts: RenderOptions) {
    let sheet = Sheet(opts.theme);
    let options = tui_markdown::Options::new(sheet).table_width(w as u16);
    let text = tui_markdown::from_str_with_options(md, &options);
    let heads = source_headings(md);
    let mut next = 0usize;

    for line in &text.lines {
        if let Some(h) = heads.get(next)
            && is_heading_line(line, h, sheet)
        {
            doc.headings.push(Heading {
                level: h.0,
                text: h.1.clone(),
                line: doc.lines.len(),
            });
            next += 1;
        }
        doc.lines.extend(wrap_line(line, w));
    }
}

fn push_diagram(doc: &mut RenderedDoc, pic: &str, kind: &str, w: usize, theme: Theme) {
    if doc.lines.last().is_some_and(|l| plain_of(l).trim() != "") {
        doc.lines.push(Line::from(""));
    }
    let frame = theme.frame();
    let title = format!("── mermaid: {kind} ");
    doc.lines.push(Line::from(vec![
        Span::styled(title.clone(), frame),
        Span::styled("─".repeat(w.saturating_sub(title.width()).max(1)), frame),
    ]));
    for l in pic.lines() {
        doc.lines
            .extend(wrap_line(&Line::styled(l.to_string(), frame), w));
    }
    doc.lines.push(Line::from(""));
}

/// 用 pulldown-cmark 取标题（与 tui-markdown 同一套解析选项），拿到顺序化的 (level, text)。
fn source_headings(md: &str) -> Vec<(u8, String)> {
    use pulldown_cmark::{Options as PdOptions, Parser, Tag};
    let mut p = PdOptions::empty();
    p.insert(PdOptions::ENABLE_GFM);
    p.insert(PdOptions::ENABLE_TABLES);
    p.insert(PdOptions::ENABLE_FOOTNOTES);
    p.insert(PdOptions::ENABLE_TASKLISTS);
    p.insert(PdOptions::ENABLE_MATH);
    p.insert(PdOptions::ENABLE_YAML_STYLE_METADATA_BLOCKS);
    p.insert(PdOptions::ENABLE_HEADING_ATTRIBUTES);

    let mut out = Vec::new();
    let mut depth = 0u8;
    let mut cur = String::new();
    for ev in Parser::new_ext(md, p) {
        match ev {
            pulldown_cmark::Event::Start(Tag::Heading { level, .. }) => {
                depth = level as u8;
                cur.clear();
            }
            pulldown_cmark::Event::End(pulldown_cmark::TagEnd::Heading(_)) => {
                out.push((depth, std::mem::take(&mut cur).trim().to_string()));
                depth = 0;
            }
            pulldown_cmark::Event::Text(t) if depth > 0 => cur.push_str(&t),
            pulldown_cmark::Event::Code(c) if depth > 0 => cur.push_str(&c),
            pulldown_cmark::Event::SoftBreak if depth > 0 => cur.push(' '),
            _ => {}
        }
    }
    out
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

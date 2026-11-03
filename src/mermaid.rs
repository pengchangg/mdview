//! 把 markdown 源码切成「普通段 / mermaid 图段」。
//!
//! 切段是必须的：`tui-markdown` 会把 ```mermaid 当普通代码块渲染，
//! 而图必须先离开 markdown 流水线交给 `mermaid-text` 画成字符图。

/// 一个渲染单元：markdown 或 mermaid 源码。
#[derive(Debug, PartialEq, Eq)]
pub enum Seg {
    Md(String),
    Diagram(String),
}

/// 围栏起始行 → (围栏标记, info string)。
fn open_fence(line: &str) -> Option<(String, String)> {
    let t = line.trim_start();
    for m in ["```", "~~~"] {
        if let Some(rest) = t.strip_prefix(m) {
            return Some((m.to_string(), rest.trim().to_string()));
        }
    }
    None
}

fn is_mermaid(info: &str) -> bool {
    info.split_whitespace()
        .next()
        .is_some_and(|w| w.eq_ignore_ascii_case("mermaid"))
}

fn closes_fence(line: &str, marker: &str) -> bool {
    line.trim_start()
        .strip_prefix(marker)
        .is_some_and(|rest| rest.trim().is_empty())
}

fn flush(out: &mut Vec<Seg>, buf: &mut String) {
    if !buf.is_empty() {
        out.push(Seg::Md(std::mem::take(buf)));
    }
}

/// 按 mermaid 围栏切段；非 mermaid 代码块整体留在 Md 段里（内部的 ``` 不再当图看）。
pub fn split(src: &str) -> Vec<Seg> {
    #[derive(Default)]
    struct St {
        fence: Option<String>,
        diagram: Option<String>,
    }
    let mut out: Vec<Seg> = Vec::new();
    let mut buf = String::new();
    let mut st = St::default();

    for line in src.lines() {
        match &st.fence {
            Some(marker) => {
                let closing = closes_fence(line, marker);
                if let Some(d) = &mut st.diagram {
                    if !closing {
                        d.push_str(line); // 闭合围栏不进图：mermaid 解析器会把 ``` 当成一个节点
                        d.push('\n');
                    }
                } else {
                    buf.push_str(line);
                    buf.push('\n'); // 非 mermaid 代码块连围栏一起留给 markdown
                }
                if closing {
                    if let Some(d) = st.diagram.take() {
                        out.push(Seg::Diagram(d));
                    }
                    st.fence = None;
                }
            }
            None => match open_fence(line) {
                Some((marker, info)) if is_mermaid(&info) => {
                    flush(&mut out, &mut buf); // 图之前的正文先成段，顺序才对
                    st.fence = Some(marker);
                    st.diagram = Some(String::new());
                }
                Some((marker, _)) => {
                    st.fence = Some(marker);
                    buf.push_str(line);
                    buf.push('\n');
                }
                None => {
                    buf.push_str(line);
                    buf.push('\n');
                }
            },
        }
    }
    if let Some(d) = st.diagram {
        out.push(Seg::Diagram(d)); // 未闭合围栏：仍当作图渲染，和 markdown 的宽容行为一致
    }
    flush(&mut out, &mut buf);
    out
}

/// 图种标签，用于图块表头，如 `flowchart TD` / `sequenceDiagram`。
pub fn kind_label(src: &str) -> String {
    for line in src.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("%%") {
            continue;
        }
        let mut words = t.split_whitespace().take(2);
        let a = words.next().unwrap_or("");
        return match words.next() {
            Some(b) if !b.starts_with(';') && !b.contains('(') => format!("{a} {b}"),
            _ => a.to_string(),
        };
    }
    "mermaid".into()
}

/// 画成字符图；失败返回 `None`（调用方退回代码块显示）。
pub fn render(src: &str, width: usize, ascii: bool) -> Option<String> {
    let r = if ascii {
        mermaid_text::render_ascii_with_width(src, Some(width))
    } else {
        mermaid_text::render_with_width(src, Some(width))
    };
    match r {
        Ok(s) if s.trim().is_empty() => None,
        Ok(s) => Some(s),
        Err(_) => None,
    }
}

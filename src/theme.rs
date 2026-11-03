//! 调色板：终端配色 + markdown 样式表。
//!
//! 两套主题（dark/light）共用一份 `StyleSheet` 实现；`color == false`（`NO_COLOR`、
//! `TERM=dumb`）时全部退化为默认色，只保留结构与 `#` 标记。

use ratatui::style::{Color, Modifier, Style};
use tui_markdown::StyleSheet;

use crate::cli::ThemeMode;

const DARK: [Color; 6] = [
    Color::Rgb(130, 215, 255),
    Color::Rgb(130, 215, 255),
    Color::Cyan,
    Color::LightBlue,
    Color::LightMagenta,
    Color::LightYellow,
];
const LIGHT: [Color; 6] = [
    Color::Rgb(0, 70, 140),
    Color::Rgb(0, 70, 140),
    Color::Rgb(0, 96, 140),
    Color::Rgb(70, 70, 160),
    Color::Rgb(120, 40, 130),
    Color::Rgb(140, 80, 0),
];

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub light: bool,
    pub color: bool,
    /// 终端不支持 Unicode 时用 ASCII 画 mermaid 边框。
    pub ascii: bool,
}

impl Theme {
    pub fn detect(mode: ThemeMode) -> Self {
        let color = std::env::var_os("NO_COLOR").is_none_or(|v| !v.is_empty())
            && std::env::var("TERM").map(|t| t != "dumb").unwrap_or(true);
        let ascii = std::env::var("TERM").is_ok_and(|t| t == "dumb");
        let light = match mode {
            ThemeMode::Light => true,
            ThemeMode::Dark => false,
            // 终端背景色无法直接查询，只能靠 COLORFGBG（末字段是背景色号）猜。
            ThemeMode::Auto => std::env::var("COLORFGBG")
                .ok()
                .and_then(|v| v.rsplit(';').next().and_then(|b| b.parse::<u8>().ok()))
                .is_some_and(|bg| bg >= 7 && bg != 8),
        };
        Self {
            light,
            color,
            ascii,
        }
    }

    fn pal(&self) -> &'static [Color; 6] {
        if self.light { &LIGHT } else { &DARK }
    }

    fn fg(&self, i: usize) -> Style {
        if self.color {
            Style::new().fg(self.pal()[i])
        } else {
            Style::new()
        }
    }

    pub fn status_bar(&self) -> Style {
        if !self.color {
            return Style::new();
        }
        Style::new()
            .fg(if self.light {
                Color::White
            } else {
                Color::Black
            })
            .bg(if self.light {
                Color::Rgb(60, 60, 60)
            } else {
                Color::Rgb(225, 225, 225)
            })
    }

    pub fn search_hit(&self) -> Style {
        if !self.color {
            return Style::new();
        }
        Style::new().fg(Color::Black).bg(Color::Rgb(240, 210, 90))
    }

    pub fn search_current(&self) -> Style {
        if !self.color {
            return Style::new();
        }
        Style::new()
            .fg(Color::Black)
            .bg(Color::Rgb(255, 140, 60))
            .add_modifier(Modifier::BOLD)
    }

    pub fn outline_current(&self) -> Style {
        if !self.color {
            return Style::new().add_modifier(Modifier::BOLD);
        }
        Style::new().bg(Color::Rgb(
            if self.light { 210 } else { 55 },
            if self.light { 225 } else { 75 },
            if self.light { 245 } else { 110 },
        ))
    }

    pub fn outline_selected(&self) -> Style {
        if !self.color {
            return Style::new().add_modifier(Modifier::REVERSED);
        }
        Style::new().add_modifier(Modifier::REVERSED)
    }

    pub fn frame(&self) -> Style {
        if !self.color {
            return Style::new();
        }
        self.fg(2).add_modifier(Modifier::DIM)
    }

    pub fn text(&self) -> Style {
        if !self.color {
            return Style::new();
        }
        Style::new().fg(if self.light {
            Color::Black
        } else {
            Color::White
        })
    }

    pub fn dim(&self) -> Style {
        Style::new().add_modifier(Modifier::DIM)
    }

    pub fn warning(&self) -> Style {
        if !self.color {
            return Style::new().add_modifier(Modifier::BOLD);
        }
        Style::new()
            .fg(Color::Rgb(255, 170, 60))
            .add_modifier(Modifier::BOLD)
    }

    pub fn scroll_thumb(&self) -> Style {
        self.fg(2).add_modifier(Modifier::BOLD)
    }

    pub fn scroll_track(&self) -> Style {
        if !self.color {
            return Style::new();
        }
        Style::new().fg(Color::DarkGray)
    }
}

/// `tui-markdown` 的样式表：只覆盖真正想改的颜色。
#[derive(Clone, Copy, Debug)]
pub struct Sheet(pub Theme);

impl StyleSheet for Sheet {
    fn heading(&self, level: u8) -> Style {
        if !self.0.color {
            return Style::new().add_modifier(Modifier::BOLD);
        }
        self.0
            .fg(level.clamp(1, 6) as usize - 1)
            .add_modifier(Modifier::BOLD)
    }

    /// 预览不是源码编辑器：标题不显示 `#`，靠颜色/加粗分层级。
    /// 大纲靠「行首 span 的 style == heading(level)」认标题（空 marker 仍是个 span）。
    fn heading_marker(&self, _level: u8) -> &str {
        ""
    }

    /// 代码块不重复贴 ``` 围栏。
    fn code_block_fence(&self) -> &str {
        ""
    }

    fn code(&self) -> Style {
        if !self.0.color {
            return Style::new();
        }
        Style::new().fg(if self.0.light {
            Color::Rgb(150, 40, 90)
        } else {
            Color::Rgb(235, 140, 160)
        })
    }

    fn link(&self) -> Style {
        if !self.0.color {
            return Style::new();
        }
        Style::new()
            .fg(Color::LightBlue)
            .add_modifier(Modifier::UNDERLINED)
    }

    fn blockquote(&self) -> Style {
        if !self.0.color {
            return Style::new();
        }
        Style::new()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::ITALIC)
    }

    fn table_border(&self) -> Style {
        self.0.fg(2)
    }

    fn table_header(&self) -> Style {
        self.0.fg(1).add_modifier(Modifier::BOLD)
    }

    fn alert(&self, _kind: tui_markdown::AlertKind) -> Style {
        self.0.fg(3)
    }
}

use eframe::egui;

#[derive(Clone, Debug, Default)]
pub struct Style {
    pub fg: Option<egui::Color32>,
    pub bg: Option<egui::Color32>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
}

#[derive(Clone, Debug)]
pub struct StyledSpan {
    pub text: String,
    pub style: Style,
}

#[derive(Clone, Debug)]
pub struct StyledLine {
    pub spans: Vec<StyledSpan>,
}

impl StyledLine {
    pub fn plain(text: &str) -> Self {
        Self {
            spans: vec![StyledSpan {
                text: text.to_string(),
                style: Style::default(),
            }],
        }
    }
}

pub struct TextBuffer {
    pub lines: Vec<StyledLine>,
    pub max_lines: usize,
    pub auto_scroll: bool,
    pub scroll_delta_lines: f32,
    pub unread_lines: usize,
}

impl TextBuffer {
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: Vec::new(),
            max_lines,
            auto_scroll: true,
            scroll_delta_lines: 0.0,
            unread_lines: 0,
        }
    }

    pub fn append_line(&mut self, line: StyledLine) {
        self.lines.push(line);
        if self.lines.len() > self.max_lines {
            self.lines.remove(0);
        }
        if !self.auto_scroll {
            self.unread_lines += 1;
        }
    }

    pub fn append_lines(&mut self, lines: Vec<StyledLine>) {
        for line in lines {
            self.append_line(line);
        }
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }
}

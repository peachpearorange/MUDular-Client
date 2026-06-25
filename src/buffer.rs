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

    pub fn plain_text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect()
    }
}

#[derive(Clone, Debug)]
pub struct Selection {
    pub anchor: (usize, usize),
    pub cursor: (usize, usize),
    pub active: bool,
    pub dragging: bool,
}

impl Default for Selection {
    fn default() -> Self {
        Self { anchor: (0, 0), cursor: (0, 0), active: false, dragging: false }
    }
}

impl Selection {
    pub fn ordered(&self) -> ((usize, usize), (usize, usize)) {
        if self.anchor <= self.cursor { (self.anchor, self.cursor) }
        else { (self.cursor, self.anchor) }
    }
}

pub struct TextBuffer {
    pub lines: Vec<StyledLine>,
    pub max_lines: usize,
    pub auto_scroll: bool,
    pub scroll_delta_lines: f32,
    pub unread_lines: usize,
    pub selection: Selection,
}

impl TextBuffer {
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: Vec::new(),
            max_lines,
            auto_scroll: true,
            scroll_delta_lines: 0.0,
            unread_lines: 0,
            selection: Selection::default(),
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
        self.selection.active = false;
    }

    pub fn selected_text(&self) -> String {
        if !self.selection.active {
            String::new()
        } else {
            let (start, end) = self.selection.ordered();
            self.lines.iter().enumerate()
                .filter(|(i, _)| *i >= start.0 && *i <= end.0)
                .map(|(i, line)| {
                    let text = line.plain_text();
                    let len = text.chars().count();
                    let col_start = if i == start.0 { start.1.min(len) } else { 0 };
                    let col_end = if i == end.0 { end.1.min(len) } else { len };
                    text.chars().skip(col_start).take(col_end.saturating_sub(col_start)).collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
    }
}

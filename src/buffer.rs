use eframe::egui;

#[derive(Clone, Debug, Default)]
pub struct Style {
  pub fg: Option<egui::Color32>,
  pub bg: Option<egui::Color32>,
  pub bold: bool,
  pub italic: bool,
  pub underline: bool,
  pub strikethrough: bool
}

#[derive(Clone, Debug)]
pub struct StyledSpan {
  pub text: String,
  pub style: Style
}

#[derive(Clone, Debug)]
pub struct StyledLine {
  pub spans: Vec<StyledSpan>
}

impl StyledLine {
  pub fn plain(text: &str) -> Self {
    Self { spans: vec![StyledSpan { text: text.to_string(), style: Style::default() }] }
  }

  pub fn is_empty(&self) -> bool { self.spans.iter().all(|span| span.text.is_empty()) }
}

pub struct TextBuffer {
  pub lines: Vec<StyledLine>,
  pub pending_line: Option<StyledLine>,
  pub max_lines: usize,
  pub auto_scroll: bool,
  pub scroll_delta_lines: f32,
  pub scroll_anim_offset: f32,
  pub scroll_anim_start_offset: f32,
  pub scroll_anim_elapsed: f32,
  pub prev_content_height: f32,
  pub unread_lines: usize
}

impl TextBuffer {
  pub fn new(max_lines: usize) -> Self {
    Self {
      lines: Vec::new(),
      pending_line: None,
      max_lines,
      auto_scroll: true,
      scroll_delta_lines: 0.0,
      scroll_anim_offset: 0.0,
      scroll_anim_start_offset: 0.0,
      scroll_anim_elapsed: 0.0,
      prev_content_height: 0.0,
      unread_lines: 0
    }
  }

  pub fn append_line(&mut self, line: StyledLine) {
    self.lines.push(line);
    if self.lines.len() > self.max_lines {
      self.lines.remove(0);
      self.prev_content_height = 0.0;
      self.scroll_anim_offset = 0.0;
      self.scroll_anim_start_offset = 0.0;
      self.scroll_anim_elapsed = 0.0;
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
    self.pending_line = None;
    self.prev_content_height = 0.0;
    self.scroll_anim_offset = 0.0;
    self.scroll_anim_start_offset = 0.0;
    self.scroll_anim_elapsed = 0.0;
  }
}

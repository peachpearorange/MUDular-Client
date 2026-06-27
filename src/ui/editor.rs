use std::collections::BTreeSet;

use eframe::egui;
use egui_code_editor::{ColorTheme, Syntax, Token, TokenType, format_token};

use crate::ansi::DEFAULT_PALETTE;

pub struct ScriptEditor {
  pub visible: bool,
  pub code: String,
  pub status_message: Option<(String, f64)>,
  syntax: Syntax,
  theme: ColorTheme,
  candidates: BTreeSet<String>,
  completion_selected: usize,
  completion_ignore_cursor: Option<egui::text::CharIndex>,
  last_cursor: egui::text::CharIndex
}

impl ScriptEditor {
  pub fn new() -> Self {
    let syntax = scheme_syntax();
    let candidates = completion_candidates(&syntax);
    Self {
      visible: false,
      code: String::new(),
      status_message: None,
      syntax,
      theme: color_theme_from_palette(&DEFAULT_PALETTE, default_bg(), default_fg()),
      candidates,
      completion_selected: 0,
      completion_ignore_cursor: None,
      last_cursor: egui::text::CharIndex::default()
    }
  }

  pub fn open(&mut self, code: &str) {
    self.code = code.to_string();
    self.visible = true;
  }

  pub fn set_theme(
    &mut self,
    palette: &[egui::Color32; 16],
    bg: Option<egui::Color32>,
    fg: Option<egui::Color32>
  ) {
    self.theme = color_theme_from_palette(palette, bg.unwrap_or(default_bg()), fg.unwrap_or(default_fg()));
  }

  pub fn render(&mut self, ctx: &egui::Context) -> EditorAction {
    let mut action = EditorAction::None;

    if self.visible {
      let theme = self.theme;
      ctx.global_style_mut(|style| {
        let bg = theme.bg();
        let accent = theme.type_color(TokenType::Function);
        style.visuals.widgets.open.weak_bg_fill = accent.gamma_multiply(0.7);
        style.visuals.widgets.active.bg_fill = accent.gamma_multiply(0.7);
        style.visuals.widgets.hovered.bg_fill = accent.gamma_multiply(0.85);
        style.visuals.widgets.inactive.bg_fill =
          if is_dark(bg) { bg.gamma_multiply(1.2) } else { bg.gamma_multiply(0.8) };
      });

      let mut visible = self.visible;
      egui::Window::new("Script Editor")
        .default_size([600.0, 500.0])
        .resizable(true)
        .collapsible(true)
        .open(&mut visible)
        .show(ctx, |ui| {
          ui.horizontal(|ui| {
            if crate::ui::term_button(ui, "Copy to Clipboard").clicked() {
              crate::ui::copy_to_clipboard(ui.ctx(), self.code.clone());
              self.status_message =
                Some(("Copied!".into(), ui.input(|input| input.time)));
            }
            if crate::ui::term_button(ui, "Save & Reload").clicked() {
              action = EditorAction::SaveAndReload(self.code.clone());
            }
            if let Some((ref msg, when)) = self.status_message {
              if ui.input(|input| input.time) - when < 3.0 {
                ui.label(msg);
              } else {
                self.status_message = None;
              }
            }
          });
          ui.separator();

          let syntax = &self.syntax;
          let fontsize = ui
            .style()
            .text_styles
            .get(&egui::TextStyle::Monospace)
            .map(|f| f.size)
            .unwrap_or(13.0);
          let row_height = ui.text_style_height(&egui::TextStyle::Monospace);
          let desired_rows = (ui.available_height() / row_height) as usize;

          let output = egui::Frame::new().fill(theme.bg()).show(ui, |ui| {
            theme.modify_style(ui, fontsize);
            egui::ScrollArea::horizontal()
              .auto_shrink([true, false])
              .show(ui, |ui| {
                egui::TextEdit::multiline(&mut self.code)
                  .id_source("script editor")
                  .lock_focus(true)
                  .desired_rows(desired_rows.max(3))
                  .desired_width(f32::INFINITY)
                  .layouter(&mut |ui: &egui::Ui,
                                   text: &dyn egui::TextBuffer,
                                   _wrap_width: f32| {
                    ui.fonts_mut(|f| {
                      f.layout_job(layout_with_rainbow_parens(
                        text.as_str(),
                        syntax,
                        &theme,
                        fontsize
                      ))
                    })
                  })
                  .show(ui)
              }).inner
          }).inner;

          let max_popup_height = ui.max_rect().bottom() - output.response.rect.bottom();
          self.show_completion_popup(ctx, &output, fontsize, max_popup_height);
        });
      self.visible = visible;
    }

    action
  }

  fn show_completion_popup(
    &mut self,
    ctx: &egui::Context,
    output: &egui::text_edit::TextEditOutput,
    fontsize: f32,
    max_popup_height: f32
  ) {
    if !output.response.has_focus() {
      return;
    }

    let Some(cursor_range) = output.state.cursor.char_range() else { return };
    let cursor = cursor_range.primary;
    let char_count = output.galley.job.text.chars().count();
    let cursor = egui::text::CharIndex(cursor.index.0.min(char_count));

    let next_char = output.galley.text().chars().nth(cursor.0);
    let next_allows = next_char.is_none_or(|c| !is_word_body(c));
    if !next_allows {
      return;
    }

    if self.completion_ignore_cursor == Some(cursor) {
      return;
    }

    if cursor != self.last_cursor {
      self.last_cursor = cursor;
      self.completion_selected = 0;
      self.completion_ignore_cursor = None;
    }

    let prefix = completion_prefix(&self.code, cursor.0);
    if prefix.is_empty() {
      return;
    }

    let completions: Vec<_> = self
      .candidates
      .iter()
      .filter(|c| c.starts_with(&prefix) && c.len() > prefix.len())
      .cloned()
      .collect();
    if completions.is_empty() {
      return;
    }

    let last = completions.len().saturating_sub(1);
    let mut insert = None;
    ctx.input_mut(|i| {
      if i.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
        self.completion_ignore_cursor = Some(cursor);
      } else if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
        self.completion_selected = if self.completion_selected == last { 0 } else { self.completion_selected + 1 };
      } else if i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
        self.completion_selected = if self.completion_selected == 0 { last } else { self.completion_selected - 1 };
      } else if i.consume_key(egui::Modifiers::NONE, egui::Key::Tab)
        || i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
      {
        insert = completions.get(self.completion_selected).cloned();
      }
    });

    if let Some(word) = insert {
      let suffix = &word[prefix.len()..];
      ctx.input_mut(|i| i.events.push(egui::Event::Paste(suffix.to_string())));
      self.completion_ignore_cursor = Some(cursor);
      return;
    }

    let cursor_pos = output.galley.pos_from_cursor(egui::text::CCursor::new(cursor));
    let cursor_rect = cursor_pos.translate(output.response.rect.left_top().to_vec2());

    egui::Popup::new(
      egui::Id::new("script_editor_completer"),
      ctx.clone(),
      cursor_rect,
      output.response.layer_id
    )
    .kind(egui::PopupKind::Tooltip)
    .frame(egui::Frame::popup(&ctx.global_style()).fill(self.theme.bg()))
    .sense(egui::Sense::empty())
    .show(|ui| {
      ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
      egui::ScrollArea::vertical()
        .auto_shrink([true, false])
        .max_height(max_popup_height.max(50.0))
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .show(ui, |ui| {
          for (i, completion) in completions.iter().enumerate() {
            let token_type = if self.syntax.is_keyword(completion) {
              TokenType::Keyword
            } else if self.syntax.is_special(completion) {
              TokenType::Special
            } else if self.syntax.is_type(completion) {
              TokenType::Type
            } else {
              TokenType::Literal
            };
            let fmt = format_token(&self.theme, fontsize, token_type);
            let job = egui::text::LayoutJob::single_section(completion.clone(), fmt);
            let selected = i == self.completion_selected;
            let button = ui.add(
              egui::Button::new(job)
                .sense(egui::Sense::click())
                .frame(true)
                .fill(if selected {
                  self.theme.type_color(TokenType::Function).gamma_multiply(0.3)
                } else {
                  egui::Color32::TRANSPARENT
                })
            );
            if selected {
              ui.scroll_to_rect(button.rect, None);
            }
            if button.clicked() {
              let suffix = &completion[prefix.len()..];
              ctx.input_mut(|input| input.events.push(egui::Event::Paste(suffix.to_string())));
              self.completion_ignore_cursor = Some(cursor);
            }
          }
        });
    });
  }
}

pub enum EditorAction {
  None,
  SaveAndReload(String)
}

fn default_bg() -> egui::Color32 { egui::Color32::from_rgb(30, 30, 30) }

fn default_fg() -> egui::Color32 { egui::Color32::from_rgb(220, 220, 220) }

fn is_dark(c: egui::Color32) -> bool {
  let [r, g, b, _] = c.to_array();
  (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) < 128.0
}

fn is_word_body(c: char) -> bool {
  c.is_alphanumeric() || c == '_' || c == '-' || c == '/' || c == '+' || c == '?' || c == '!'
}

fn completion_prefix(text: &str, cursor: usize) -> String {
  let chars: Vec<char> = text.chars().collect();
  let cursor = cursor.min(chars.len());
  let start = chars[..cursor]
    .iter()
    .enumerate()
    .rfind(|(_, c)| !is_word_body(**c))
    .map(|(i, _)| i + 1)
    .unwrap_or(0);
  chars[start..cursor].iter().collect()
}

fn color_theme_from_palette(
  palette: &[egui::Color32; 16],
  bg: egui::Color32,
  fg: egui::Color32
) -> ColorTheme {
  let leak = |c: egui::Color32| -> &'static str {
    Box::leak(format!("{:02x}{:02x}{:02x}", c.r(), c.g(), c.b()).into_boxed_str())
  };
  let dim = |c: egui::Color32| c.gamma_multiply(0.65);

  ColorTheme {
    name: "dynamic",
    dark: is_dark(bg),
    bg: leak(bg),
    cursor: leak(fg),
    selection: leak(dim(fg)),
    comments: leak(palette[8]),
    functions: leak(palette[11]),
    keywords: leak(palette[9]),
    literals: leak(fg),
    numerics: leak(palette[13]),
    punctuation: leak(palette[7]),
    strs: leak(palette[10]),
    types: leak(palette[14]),
    special: leak(palette[12])
  }
}

fn layout_with_rainbow_parens(
  text: &str,
  syntax: &Syntax,
  theme: &ColorTheme,
  fontsize: f32
) -> egui::text::LayoutJob {
  let mut job = egui::text::LayoutJob::default();
  let mut tokenizer = Token::default();
  let mut depth = 0isize;

  let rainbow_colors = [
    theme.type_color(TokenType::Function),
    theme.type_color(TokenType::Keyword),
    theme.type_color(TokenType::Type),
    theme.type_color(TokenType::Str('"')),
    theme.type_color(TokenType::Numeric(false)),
    theme.type_color(TokenType::Special)
  ];

  for token in tokenizer.tokens(syntax, text) {
    if let TokenType::Punctuation(_) = token.ty() {
      for c in token.buffer().chars() {
        let color = match c {
          '(' => {
            let color = rainbow_colors[(depth as usize).rem_euclid(rainbow_colors.len())];
            depth += 1;
            color
          }
          ')' => {
            depth = (depth - 1).max(0);
            rainbow_colors[(depth as usize).rem_euclid(rainbow_colors.len())]
          }
          _ => theme.type_color(token.ty())
        };
        let format = egui::text::TextFormat::simple(
          egui::FontId::monospace(fontsize),
          color
        );
        job.append(&c.to_string(), 0.0, format);
      }
    } else {
      job.append(token.buffer(), 0.0, format_token(theme, fontsize, token.ty()));
    }
  }

  job
}

fn completion_candidates(syntax: &Syntax) -> BTreeSet<String> {
  let mut candidates = BTreeSet::new();
  for word in syntax.keywords.iter().chain(syntax.types.iter()).chain(syntax.special.iter()) {
    candidates.insert(word.to_string());
  }
  candidates
}

fn scheme_syntax() -> Syntax {
  let mut keywords = BTreeSet::from([
    "define",
    "lambda",
    "if",
    "cond",
    "else",
    "let",
    "let*",
    "letrec",
    "begin",
    "set!",
    "and",
    "or",
    "not",
    "when",
    "unless",
    "case",
    "do",
    "delay",
    "force",
    "for-each",
    "map",
    "filter",
    "foldl",
    "foldr",
    "apply",
    "eval",
    "quote",
    "quasiquote",
    "unquote",
    "unquote-splicing",
    "car",
    "cdr",
    "cons",
    "list",
    "append",
    "reverse",
    "length",
    "null?",
    "pair?",
    "list?",
    "eq?",
    "eqv?",
    "equal?",
    "number?",
    "string?",
    "symbol?",
    "boolean?",
    "procedure?",
    "display",
    "newline",
    "write",
    "read",
    "format",
    "to-string",
    "string-join",
    "string-replace",
    "string-contains?",
    "starts-with?",
    "hash",
    "hash-ref",
    "hash-set",
    "hash-insert",
    "hash-remove",
    "hash-contains?",
    "hash-keys->list",
    "hash?",
    "void",
    "void?",
    "trim",
    "round",
    "floor",
    "ceiling",
    "truncate",
    "sin",
    "cos",
    "tan",
    "asin",
    "acos",
    "atan",
    "sqrt",
    "expt",
    "log",
    "exp",
    "abs",
    "max",
    "min",
    "modulo",
    "remainder",
    "quotient",
    "random",
    "time",
    "current-milliseconds"
  ]);

  let api = [
    "mud/send",
    "mud/reconnect",
    "mud/scroll-up",
    "mud/scroll-down",
    "mud/capture-key",
    "mud/keymap",
    "mud/pane",
    "mud/pane-print",
    "mud/pane-clear",
    "mud/gauge",
    "mud/layout",
    "mud/status",
    "mud/profile",
    "mud/profile*",
    "mud/on",
    "mud/option",
    "mud/load-theme",
    "mud/themes",
    "mud/fonts",
    "mud/strip-ansi",
    "mud/regexp-match?",
    "mud/send-gmcp",
    "mud/msdp-report",
    "mud/msdp-send",
    "mud/msdp-list"
  ];
  for word in api {
    keywords.insert(word);
  }

  let mut syntax = Syntax {
    language: "Scheme",
    case_sensitive: true,
    comment: ";",
    comment_multiline: ["#|", "|#"],
    quotes: BTreeSet::from(['\'', '"', '`']),
    word_start: BTreeSet::from(['?', '!', '-', '+', '*', '/', '<', '>', '=']),
    hyperlinks: BTreeSet::new(),
    keywords,
    types: BTreeSet::from(["hash", "list", "vector", "string", "number", "integer", "float"]),
    special: BTreeSet::from(["#t", "#f", "true", "false", "nil", "'()"]),
    patch: Default::default()
  };

  for name in crate::themes::theme_names() {
    let sym = Box::leak(crate::themes::theme_symbol(name).into_boxed_str());
    syntax.keywords.insert(sym);
  }

  syntax
}

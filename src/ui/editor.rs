use std::collections::BTreeSet;

use eframe::egui;
use egui_code_editor::{ColorTheme, Completer, Syntax, Token, TokenType, format_token};

use crate::ansi::DEFAULT_PALETTE;

pub struct ScriptEditor {
  pub visible: bool,
  pub code: String,
  pub status_message: Option<(String, f64)>,
  syntax: Syntax,
  theme: ColorTheme,
  completer: Completer
}

impl ScriptEditor {
  pub fn new() -> Self {
    let syntax = scheme_syntax_with_themes();
    Self {
      visible: false,
      code: String::new(),
      status_message: None,
      completer: Completer::new_with_syntax(&syntax),
      theme: color_theme_from_palette(&DEFAULT_PALETTE, default_bg(), default_fg()),
      syntax
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
      egui::Window::new("Script Editor")
        .default_size([600.0, 500.0])
        .resizable(true)
        .collapsible(true)
        .open(&mut self.visible)
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

          let theme = self.theme;
          let syntax = &self.syntax;
          let fontsize = ui
            .style()
            .text_styles
            .get(&egui::TextStyle::Monospace)
            .map(|f| f.size)
            .unwrap_or(13.0);

          egui::Frame::new().fill(theme.bg()).show(ui, |ui| {
            theme.modify_style(ui, fontsize);
            let style = ui.style_mut();
            let bg = theme.bg();
            let accent = theme.type_color(TokenType::Function);
            style.visuals.widgets.active.bg_fill = accent.gamma_multiply(0.7);
            style.visuals.widgets.hovered.bg_fill = accent.gamma_multiply(0.85);
            style.visuals.widgets.inactive.bg_fill =
              if is_dark(bg) { bg.gamma_multiply(1.2) } else { bg.gamma_multiply(0.8) };
            egui::ScrollArea::horizontal().show(ui, |ui| {
              self.completer.show_on_text_widget(
                ui,
                syntax,
                &theme,
                |ui| {
                  egui::TextEdit::multiline(&mut self.code)
                    .id_source("script editor")
                    .lock_focus(true)
                    .desired_rows(30)
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
                }
              );
            });
          });
        });
    }

    action
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

fn scheme_syntax_with_themes() -> Syntax {
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

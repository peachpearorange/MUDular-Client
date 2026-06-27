use std::collections::BTreeSet;

use eframe::egui;
use egui_code_editor::{CodeEditor, ColorTheme, Completer, Syntax};

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
    let syntax = scheme_syntax();
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

          CodeEditor::default()
            .id_source("script editor")
            .with_rows(30)
            .with_ui_fontsize(ui)
            .with_theme(self.theme)
            .with_numlines(false)
            .desired_width(f32::INFINITY)
            .vscroll(true)
            .show_with_completer(ui, &mut self.code, &self.syntax, &mut self.completer);
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

fn color_theme_from_palette(
  palette: &[egui::Color32; 16],
  bg: egui::Color32,
  fg: egui::Color32
) -> ColorTheme {
  let leak = |c: egui::Color32| -> &'static str {
    Box::leak(format!("{:02x}{:02x}{:02x}", c.r(), c.g(), c.b()).into_boxed_str())
  };
  let is_dark = |c: egui::Color32| {
    let [r, g, b, _] = c.to_array();
    (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) < 128.0
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

  Syntax {
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
  }
}

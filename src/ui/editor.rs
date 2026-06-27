use std::collections::BTreeSet;

use eframe::egui;
use egui_code_editor::{CodeEditor, ColorTheme, Syntax};

pub struct ScriptEditor {
  pub visible: bool,
  pub code: String,
  pub status_message: Option<(String, f64)>,
  syntax: Syntax
}

impl ScriptEditor {
  pub fn new() -> Self {
    Self { visible: false, code: String::new(), status_message: None, syntax: scheme_syntax() }
  }

  pub fn open(&mut self, code: &str) {
    self.code = code.to_string();
    self.visible = true;
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
            .with_theme(ColorTheme::GRUVBOX)
            .with_numlines(true)
            .desired_width(f32::INFINITY)
            .vscroll(true)
            .show(ui, &mut self.code, &self.syntax);
        });
    }

    action
  }
}

pub enum EditorAction {
  None,
  SaveAndReload(String)
}

fn scheme_syntax() -> Syntax {
  Syntax {
    language: "Scheme",
    case_sensitive: true,
    comment: ";",
    comment_multiline: ["#|", "|#"],
    quotes: BTreeSet::from(['\'', '"', '`']),
    word_start: BTreeSet::from(['?', '!', '-', '+', '*', '/', '<', '>', '=']),
    hyperlinks: BTreeSet::new(),
    keywords: BTreeSet::from([
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
    ]),
    types: BTreeSet::from(["hash", "list", "vector", "string", "number", "integer", "float"]),
    special: BTreeSet::from(["#t", "#f", "true", "false", "nil", "'()"]),
    patch: Default::default()
  }
}

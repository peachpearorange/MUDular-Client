use eframe::egui;

pub struct ScriptEditor {
  pub visible: bool,
  pub code: String,
  pub status_message: Option<(String, std::time::Instant)>
}

impl ScriptEditor {
  pub fn new() -> Self {
    Self { visible: false, code: String::new(), status_message: None }
  }

  pub fn open(&mut self, code: &str) {
    self.code = code.to_string();
    self.visible = true;
  }

  pub fn render(&mut self, ctx: &egui::Context) -> EditorAction {
    let mut action = EditorAction::None;

    if !self.visible {
      return action;
    }

    egui::Window::new("Script Editor")
      .default_size([600.0, 500.0])
      .resizable(true)
      .collapsible(true)
      .open(&mut self.visible)
      .show(ctx, |ui| {
        ui.horizontal(|ui| {
          if crate::ui::term_button(ui, "Copy to Clipboard").clicked() {
            crate::ui::copy_to_clipboard(ui.ctx(), self.code.clone());
            self.status_message = Some(("Copied!".into(), std::time::Instant::now()));
          }
          if crate::ui::term_button(ui, "Save & Reload").clicked() {
            action = EditorAction::SaveAndReload(self.code.clone());
          }
          if let Some((ref msg, when)) = self.status_message {
            if when.elapsed().as_secs() < 3 {
              ui.label(msg);
            } else {
              self.status_message = None;
            }
          }
        });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
          ui.add(
            egui::TextEdit::multiline(&mut self.code)
              .font(egui::TextStyle::Monospace)
              .desired_width(f32::INFINITY)
              .desired_rows(30)
          );
        });
      });

    action
  }
}

pub enum EditorAction {
  None,
  SaveAndReload(String)
}

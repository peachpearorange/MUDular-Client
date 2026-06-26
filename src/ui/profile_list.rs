use eframe::egui;

use crate::profile::Profile;

pub enum ProfileAction {
  None,
  Connect(usize),
  EditScript(usize),
  ShowTemplatePicker,
  DeleteProfile(usize),
  RenameProfile(usize)
}

pub fn render_profile_list(
  ui: &mut egui::Ui,
  profiles: &[Profile],
  mssp_info: &std::collections::HashMap<
    String,
    std::collections::HashMap<String, String>
  >
) -> ProfileAction {
  let mut action = ProfileAction::None;

  ui.heading("MUDular Client");
  ui.add_space(10.0);

  if crate::ui::term_button(ui, "+ New Profile").clicked() {
    action = ProfileAction::ShowTemplatePicker;
  }
  ui.add_space(10.0);

  if profiles.is_empty() {
    ui.label("No profiles yet. Click '+ New Profile' to get started.");
  } else {
    ui.separator();
    ui.add_space(4.0);
    egui::ScrollArea::vertical().show(ui, |ui| {
      for (i, profile) in profiles.iter().enumerate() {
        ui.group(|ui| {
          ui.horizontal(|ui| {
            ui.vertical(|ui| {
              ui.label(egui::RichText::new(&profile.name).strong());
              let key = format!("{}:{}", profile.host, profile.port);
              let status = mssp_info
                .get(&key)
                .and_then(|info| info.get("PLAYERS"))
                .map(|p| format!("  ({p} online)"))
                .unwrap_or_default();
              ui.label(format!("{}:{}{status}", profile.host, profile.port));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
              if crate::ui::term_button(ui, "Connect").clicked() {
                action = ProfileAction::Connect(i);
              }
              if crate::ui::term_button(ui, "Edit Script").clicked() {
                action = ProfileAction::EditScript(i);
              }
              if crate::ui::term_button(ui, "Rename").clicked() {
                action = ProfileAction::RenameProfile(i);
              }
              if crate::ui::term_button(ui, "Delete").clicked() {
                action = ProfileAction::DeleteProfile(i);
              }
            });
          });
        });
        ui.add_space(4.0);
      }
    });
  }

  action
}

pub enum TemplateAction {
  None,
  CreateFromTemplate(usize),
  CustomProfile,
  Cancel
}

pub fn render_template_picker(
  ctx: &egui::Context,
  templates: &[Profile],
  open: &mut bool
) -> TemplateAction {
  let mut action = TemplateAction::None;
  egui::Window::new("New Profile")
    .collapsible(false)
    .resizable(false)
    .open(open)
    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
    .show(ctx, |ui| {
      ui.set_min_width(350.0);
      ui.label("Choose a game template:");
      ui.add_space(8.0);
      for (i, template) in templates.iter().enumerate() {
        ui.horizontal(|ui| {
          if crate::ui::term_button(ui, &template.name).clicked() {
            action = TemplateAction::CreateFromTemplate(i);
          }
          ui.label(
            egui::RichText::new(format!("{}:{}", template.host, template.port))
              .color(egui::Color32::from_gray(140))
          );
        });
        ui.add_space(2.0);
      }
      ui.add_space(8.0);
      ui.separator();
      ui.add_space(4.0);
      if crate::ui::term_button(ui, "Custom").clicked() {
        action = TemplateAction::CustomProfile;
      }
    });
  if !*open {
    action = TemplateAction::Cancel;
  }
  action
}

use eframe::egui;

use crate::{buffer::TextBuffer,
            scripting::{Layout, LayoutDir},
            ui::panes::render_pane};

pub fn render_layout(
  ui: &mut egui::Ui,
  layout: &Layout,
  panes: &mut std::collections::HashMap<String, TextBuffer>
) {
  let total_weight: f32 = layout.entries.iter().map(|e| e.weight).sum();
  if total_weight <= 0.0 {
    return;
  }

  let rect = ui.available_rect_before_wrap();
  let divider_stroke = ui.visuals().widgets.noninteractive.bg_stroke;
  let divider_width = 1.0_f32;

  match layout.direction {
    LayoutDir::Horizontal => {
      let num_dividers = layout.entries.len().saturating_sub(1) as f32;
      let usable_width = rect.width() - num_dividers * divider_width;
      let mut x = rect.left();

      for (i, entry) in layout.entries.iter().enumerate() {
        if i > 0 {
          ui.painter().vline(x, rect.y_range(), divider_stroke);
          x += divider_width;
        }
        let width = usable_width * (entry.weight / total_weight);
        let pane_rect = egui::Rect::from_min_size(
          egui::pos2(x, rect.top()),
          egui::vec2(width, rect.height())
        );
        let mut child_ui =
          ui.new_child(egui::UiBuilder::new().max_rect(pane_rect).id_salt(&entry.pane));
        if let Some(buf) = panes.get_mut(&entry.pane) {
          render_pane(&mut child_ui, &entry.pane, buf);
        }
        x += width;
      }
      ui.advance_cursor_after_rect(rect);
    }
    LayoutDir::Vertical => {
      let num_dividers = layout.entries.len().saturating_sub(1) as f32;
      let usable_height = rect.height() - num_dividers * divider_width;
      let mut y = rect.top();

      for (i, entry) in layout.entries.iter().enumerate() {
        if i > 0 {
          ui.painter().hline(rect.x_range(), y, divider_stroke);
          y += divider_width;
        }
        let height = usable_height * (entry.weight / total_weight);
        let pane_rect = egui::Rect::from_min_size(
          egui::pos2(rect.left(), y),
          egui::vec2(rect.width(), height)
        );
        let mut child_ui =
          ui.new_child(egui::UiBuilder::new().max_rect(pane_rect).id_salt(&entry.pane));
        if let Some(buf) = panes.get_mut(&entry.pane) {
          render_pane(&mut child_ui, &entry.pane, buf);
        }
        y += height;
      }
      ui.advance_cursor_after_rect(rect);
    }
  }
}

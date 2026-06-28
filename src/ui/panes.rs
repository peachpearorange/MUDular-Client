use {eframe::{egui,
              egui::{Color32, ScrollArea, TextFormat}},
     egui::{text::LayoutJob, text_selection::LabelSelectionState}};

use crate::buffer::{Style, StyledLine, TextBuffer};

pub fn render_pane(ui: &mut egui::Ui, _name: &str, buffer: &mut TextBuffer) {
  let pane_rect = ui.max_rect();

  let font_id = ui
    .style()
    .text_styles
    .get(&egui::TextStyle::Monospace)
    .cloned()
    .unwrap_or_else(|| egui::FontId::monospace(13.0));
  let available_width = ui.available_width();
  let default_color = ui.visuals().text_color();
  let animating_scroll = buffer.auto_scroll && buffer.scroll_anim_offset > 0.5;
  let animated_scroll_offset =
    (buffer.prev_content_height - pane_rect.height() - buffer.scroll_anim_offset)
      .max(0.0);

  let scroll = ScrollArea::vertical()
    .auto_shrink([false; 2])
    .stick_to_bottom(buffer.auto_scroll && !animating_scroll);
  let scroll = if animating_scroll {
    scroll.vertical_scroll_offset(animated_scroll_offset)
  } else {
    scroll
  };

  let output = scroll.show(ui, |ui| {
    for line in &buffer.lines {
      render_styled_line(ui, line, available_width, &font_id, default_color);
    }
    let y_before = ui.cursor().min.y;
    if let Some(line) = &buffer.pending_line {
      render_styled_line(ui, line, available_width, &font_id, default_color);
    }
    ui.cursor().min.y - y_before
  });
  let pending_height = output.inner;

  let max_scroll = (output.content_size.y - output.inner_rect.height()).max(0.0);
  let at_bottom = output.state.offset.y >= max_scroll - 5.0;

  let stable_content = output.content_size.y - pending_height;
  let content_grew = stable_content - buffer.prev_stable_height;
  buffer.prev_stable_height = stable_content;
  buffer.prev_content_height = output.content_size.y;
  if content_grew > 0.5 && buffer.auto_scroll {
    buffer.scroll_anim_offset += content_grew;
    let t = (buffer.scroll_anim_elapsed / 0.55).min(1.0);
    let remaining = 1.0 - smoothstep(t);
    buffer.scroll_anim_start_offset = if remaining > 0.01 {
      buffer.scroll_anim_offset / remaining
    } else {
      buffer.scroll_anim_elapsed = 0.0;
      buffer.scroll_anim_offset
    };
  }

  if buffer.scroll_anim_offset > 0.5 && buffer.auto_scroll {
    let dt = ui.input(|i| i.predicted_dt).min(0.05);
    buffer.scroll_anim_elapsed += dt;
    let t = (buffer.scroll_anim_elapsed / 0.55).min(1.0);
    let eased = smoothstep(t);
    buffer.scroll_anim_offset = buffer.scroll_anim_start_offset * (1.0 - eased);
    if t >= 1.0 || buffer.scroll_anim_offset < 0.5 {
      buffer.scroll_anim_offset = 0.0;
      buffer.scroll_anim_start_offset = 0.0;
      buffer.scroll_anim_elapsed = 0.0;
      let mut state = output.state.clone();
      state.offset.y = max_scroll;
      state.store(ui.ctx(), output.id);
      ui.ctx().request_repaint();
    } else {
      let mut state = output.state.clone();
      state.offset.y = (max_scroll - buffer.scroll_anim_offset).max(0.0);
      state.store(ui.ctx(), output.id);
      ui.ctx().request_repaint();
    }
  }

  if buffer.scroll_delta_lines != 0.0 {
    let line_height = font_id.size + ui.spacing().item_spacing.y;
    let delta = -buffer.scroll_delta_lines * line_height;
    buffer.scroll_delta_lines = 0.0;
    let new_offset = (output.state.offset.y + delta).clamp(0.0, max_scroll);
    let mut state = output.state.clone();
    state.offset.y = new_offset;
    state.store(ui.ctx(), output.id);
    buffer.auto_scroll = new_offset >= max_scroll - 5.0;
    buffer.scroll_anim_offset = 0.0;
    buffer.scroll_anim_start_offset = 0.0;
    buffer.scroll_anim_elapsed = 0.0;
  } else if animating_scroll {
    buffer.auto_scroll = true;
  } else if at_bottom {
    buffer.auto_scroll = true;
  } else if !buffer.auto_scroll {
  } else {
    buffer.auto_scroll = at_bottom;
  }

  if buffer.auto_scroll {
    buffer.unread_lines = 0;
  }

  let in_pane = ui
    .ctx()
    .input(|i| i.pointer.latest_pos())
    .is_some_and(|pos| output.inner_rect.contains(pos));
  if in_pane && ui.input(|i| i.smooth_scroll_delta.y != 0.0) {
    buffer.auto_scroll = at_bottom;
    if !buffer.auto_scroll {
      buffer.scroll_anim_offset = 0.0;
      buffer.scroll_anim_start_offset = 0.0;
      buffer.scroll_anim_elapsed = 0.0;
    }
  }

  if !buffer.auto_scroll && buffer.unread_lines > 0 {
    let fg = ui.visuals().widgets.noninteractive.fg_stroke.color;
    let base = ui.visuals().panel_fill;
    let label = format!(" \u{2193} {} new lines ", buffer.unread_lines);
    let galley = ui.painter().layout_no_wrap(label, font_id.clone(), fg);
    let btn_size = galley.size();
    let btn_rect = egui::Rect::from_min_size(
      egui::pos2(
        pane_rect.center().x - btn_size.x / 2.0,
        pane_rect.bottom() - btn_size.y - 4.0
      ),
      btn_size
    );
    let response = ui.allocate_rect(btn_rect, egui::Sense::click());
    let bg = crate::ui::panel_button_bg(base, fg, response.hovered());
    ui.painter().rect_filled(btn_rect, 0.0, bg);
    ui.painter().galley(btn_rect.min, galley, fg);
    if response.clicked() {
      buffer.auto_scroll = true;
      buffer.unread_lines = 0;
      buffer.scroll_anim_offset = 0.0;
      buffer.scroll_anim_start_offset = 0.0;
      buffer.scroll_anim_elapsed = 0.0;
      let mut state = output.state.clone();
      state.offset.y = max_scroll;
      state.store(ui.ctx(), output.id);
    }
  }
}

fn smoothstep(t: f32) -> f32 { t * t * (3.0 - 2.0 * t) }

pub fn render_styled_line(
  ui: &mut egui::Ui,
  line: &StyledLine,
  available_width: f32,
  font_id: &egui::FontId,
  default_color: Color32
) {
  let mut job = LayoutJob::default();
  job.wrap.max_width = available_width;
  if line.spans.is_empty() {
    job.append(" ", 0.0, TextFormat { font_id: font_id.clone(), ..Default::default() });
  }
  for span in &line.spans {
    job.append(&span.text, 0.0, style_to_format(&span.style, ui.visuals(), font_id));
  }
  let galley = ui.fonts_mut(|f| f.layout_job(job));
  let desired = egui::vec2(available_width, galley.size().y);
  let (rect, response) = ui.allocate_at_least(desired, egui::Sense::click_and_drag());
  LabelSelectionState::label_text_selection(
    ui,
    &response,
    rect.min,
    galley,
    default_color,
    egui::Stroke::NONE
  );
}

fn style_to_format(
  style: &Style,
  visuals: &egui::Visuals,
  font_id: &egui::FontId
) -> TextFormat {
  let fg = style.fg.unwrap_or(visuals.text_color());
  let bg = style.bg.unwrap_or(Color32::TRANSPARENT);

  TextFormat {
    font_id: font_id.clone(),
    color: fg,
    background: bg,
    italics: style.italic,
    underline: if style.underline {
      egui::Stroke::new(1.0_f32, fg)
    } else {
      egui::Stroke::NONE
    },
    strikethrough: if style.strikethrough {
      egui::Stroke::new(1.0_f32, fg)
    } else {
      egui::Stroke::NONE
    },
    ..Default::default()
  }
}

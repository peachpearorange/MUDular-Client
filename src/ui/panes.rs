use eframe::egui;
use eframe::egui::{Color32, ScrollArea, TextFormat};
use egui::text::LayoutJob;

use crate::buffer::{Style, TextBuffer};

pub fn render_pane(ui: &mut egui::Ui, _name: &str, buffer: &mut TextBuffer) {
    let pane_rect = ui.max_rect();

    let font_id = ui.style().text_styles.get(&egui::TextStyle::Monospace)
        .cloned()
        .unwrap_or_else(|| egui::FontId::monospace(13.0));
    let char_width = ui.painter().layout_no_wrap(
        "M".to_string(), font_id.clone(), Color32::WHITE,
    ).size().x;
    let available_width = ui.available_width();

    let scroll = ScrollArea::vertical()
        .auto_shrink([false; 2])
        .stick_to_bottom(buffer.auto_scroll)
        .scroll_source(egui::containers::scroll_area::ScrollSource {
            scroll_bar: true,
            drag: false,
            mouse_wheel: true,
        });

    let output = scroll.show(ui, |ui| {
        let mut line_rects = Vec::with_capacity(buffer.lines.len());
        for line in &buffer.lines {
            let mut job = LayoutJob::default();
            job.wrap.max_width = available_width;
            if line.spans.is_empty() {
                job.append(" ", 0.0, TextFormat {
                    font_id: font_id.clone(),
                    ..Default::default()
                });
            }
            for span in &line.spans {
                let format = style_to_format(&span.style, ui.visuals(), &font_id);
                job.append(&span.text, 0.0, format);
            }
            let response = ui.label(job);
            line_rects.push(response.rect);
        }
        line_rects
    });

    let line_rects = output.inner;
    let visible_rect = output.inner_rect;

    let (pointer_pos, primary_pressed, primary_down) = ui.ctx().input(|i| (
        i.pointer.latest_pos(),
        i.pointer.primary_pressed(),
        i.pointer.primary_down(),
    ));
    let pane_layer = ui.layer_id();
    let in_pane = pointer_pos.is_some_and(|pos|
        visible_rect.contains(pos)
        && ui.ctx().layer_id_at(pos).is_none_or(|layer| layer == pane_layer)
    );

    if primary_pressed && in_pane && let Some(pos) = pointer_pos {
        let (line, col) = pos_to_buffer_coords(pos, &line_rects, char_width, buffer.lines.len());
        buffer.selection.anchor = (line, col);
        buffer.selection.cursor = (line, col);
        buffer.selection.active = false;
        buffer.selection.dragging = true;
    } else if primary_pressed {
        buffer.selection.active = false;
        buffer.selection.dragging = false;
    } else if primary_down && buffer.selection.dragging && let Some(pos) = pointer_pos {
        let (line, col) = pos_to_buffer_coords(pos, &line_rects, char_width, buffer.lines.len());
        buffer.selection.cursor = (line, col);
        if buffer.selection.anchor != buffer.selection.cursor {
            buffer.selection.active = true;
        }
    } else if !primary_down {
        buffer.selection.dragging = false;
    }

    if buffer.selection.active {
        let (start, end) = buffer.selection.ordered();
        let highlight = Color32::from_rgba_unmultiplied(80, 120, 200, 80);
        for (i, rect) in line_rects.iter().enumerate()
            .filter(|(i, _)| *i >= start.0 && *i <= end.0)
        {
            let col_start = if i == start.0 { start.1 } else { 0 };
            let col_end = if i == end.0 { end.1 } else { usize::MAX };
            let x_start = rect.left() + col_start as f32 * char_width;
            let x_end = if col_end == usize::MAX {
                pane_rect.right()
            } else {
                (rect.left() + col_end as f32 * char_width).min(pane_rect.right())
            };
            let sel_rect = egui::Rect::from_min_max(
                egui::pos2(x_start, rect.top()),
                egui::pos2(x_end, rect.bottom()),
            );
            let clipped = sel_rect.intersect(visible_rect);
            if clipped.is_positive() {
                ui.painter().rect_filled(clipped, 0.0, highlight);
            }
        }
    }

    let ctrl_c = ui.ctx().input(|i| i.modifiers.command && i.key_pressed(egui::Key::C));
    if ctrl_c && buffer.selection.active {
        let text = buffer.selected_text();
        if !text.is_empty() {
            crate::ui::copy_to_clipboard(ui.ctx(), text);
        }
    }

    let max_scroll = (output.content_size.y - output.inner_rect.height()).max(0.0);
    let at_bottom = output.state.offset.y >= max_scroll - 5.0;

    if buffer.scroll_delta_lines != 0.0 {
        let line_height = font_id.size + ui.spacing().item_spacing.y;
        let delta = -buffer.scroll_delta_lines * line_height;
        buffer.scroll_delta_lines = 0.0;
        let new_offset = (output.state.offset.y + delta).clamp(0.0, max_scroll);
        let mut state = output.state.clone();
        state.offset.y = new_offset;
        state.store(ui.ctx(), output.id);
        buffer.auto_scroll = new_offset >= max_scroll - 5.0;
    } else if at_bottom {
        buffer.auto_scroll = true;
    } else if !buffer.auto_scroll {
        // stay put
    } else {
        buffer.auto_scroll = at_bottom;
    }

    if buffer.auto_scroll {
        buffer.unread_lines = 0;
    }

    if in_pane && ui.input(|i| i.smooth_scroll_delta.y != 0.0) {
        buffer.auto_scroll = false;
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
                pane_rect.bottom() - btn_size.y - 4.0,
            ),
            btn_size,
        );
        let response = ui.allocate_rect(btn_rect, egui::Sense::click());
        let bg = if response.hovered() {
            crate::ui::lighten(base, 50)
        } else {
            crate::ui::lighten(base, 30)
        };
        ui.painter().rect_filled(btn_rect, 0.0, bg);
        ui.painter().galley(btn_rect.min, galley, fg);
        if response.clicked() {
            buffer.auto_scroll = true;
            buffer.unread_lines = 0;
            let mut state = output.state.clone();
            state.offset.y = max_scroll;
            state.store(ui.ctx(), output.id);
        }
    }
}

fn pos_to_buffer_coords(
    pos: egui::Pos2,
    line_rects: &[egui::Rect],
    char_width: f32,
    num_lines: usize,
) -> (usize, usize) {
    if line_rects.is_empty() {
        (0, 0)
    } else if pos.y < line_rects[0].top() {
        (0, ((pos.x - line_rects[0].left()) / char_width).max(0.0) as usize)
    } else {
        let line = line_rects.iter()
            .position(|r| pos.y < r.bottom())
            .unwrap_or(num_lines.saturating_sub(1));
        let col = line_rects.get(line)
            .map(|r| ((pos.x - r.left()) / char_width).max(0.0) as usize)
            .unwrap_or(0);
        (line, col)
    }
}

fn style_to_format(style: &Style, visuals: &egui::Visuals, font_id: &egui::FontId) -> TextFormat {
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

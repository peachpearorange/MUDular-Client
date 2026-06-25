use eframe::egui;
use eframe::egui::Color32;

use crate::scripting::Gauge;

pub fn render_gauges(ui: &mut egui::Ui, gauges: &[Gauge]) {
    if gauges.is_empty() {
        return;
    }

    ui.horizontal(|ui| {
        for gauge in gauges {
            render_single_gauge(ui, gauge);
            ui.add_space(4.0);
        }
    });
}

fn render_single_gauge(ui: &mut egui::Ui, gauge: &Gauge) {
    let fraction = if gauge.max > 0.0 { (gauge.current / gauge.max).clamp(0.0, 1.0) } else { 0.0 };
    let color = color_from_name(&gauge.color);
    let fg = ui.visuals().widgets.noninteractive.fg_stroke.color;
    let empty_bg = crate::ui::lighten(ui.visuals().panel_fill, 20);

    let label = format!(" {}: {}/{} ", gauge.name, gauge.current as i64, gauge.max as i64);
    let font = ui.style().text_styles.get(&egui::TextStyle::Monospace)
        .cloned()
        .unwrap_or_else(|| egui::FontId::monospace(13.0));
    let galley = ui.painter().layout_no_wrap(label, font, fg);
    let (rect, _response) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());

    let filled_width = rect.width() * fraction as f32;
    let filled_rect = egui::Rect::from_min_size(rect.min, egui::vec2(filled_width, rect.height()));
    let empty_rect = egui::Rect::from_min_max(
        egui::pos2(rect.min.x + filled_width, rect.min.y),
        rect.max,
    );

    ui.painter().rect_filled(filled_rect, 0.0, color);
    ui.painter().rect_filled(empty_rect, 0.0, empty_bg);
    ui.painter().galley(rect.min, galley, fg);
}

fn color_from_name(name: &str) -> Color32 {
    match name {
        "red" => Color32::from_rgb(160, 40, 40),
        "green" => Color32::from_rgb(40, 140, 40),
        "blue" => Color32::from_rgb(40, 80, 160),
        "yellow" => Color32::from_rgb(160, 160, 40),
        "cyan" => Color32::from_rgb(40, 160, 160),
        "magenta" => Color32::from_rgb(160, 40, 160),
        "white" => Color32::from_rgb(160, 160, 160),
        _ => Color32::from_rgb(80, 80, 80),
    }
}

pub mod panes;
pub mod gauges;
pub mod layout;
pub mod input;
pub mod editor;
pub mod profile_list;

use eframe::egui;

pub fn copy_to_clipboard(ctx: &egui::Context, text: String) {
    ctx.copy_text(text.clone());
    #[cfg(target_os = "linux")]
    std::thread::spawn(move || {
        use std::io::Write;
        let try_cmd = |cmd: &str, args: &[&str]| -> bool {
            std::process::Command::new(cmd)
                .args(args)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .ok()
                .and_then(|mut child| {
                    child.stdin.take().and_then(|mut stdin| stdin.write_all(text.as_bytes()).ok());
                    child.wait().ok()
                })
                .is_some()
        };
        let _ = try_cmd("wl-copy", &[])
            || try_cmd("xclip", &["-selection", "clipboard"])
            || try_cmd("xsel", &["--clipboard", "--input"]);
    });
}

pub fn lighten(color: egui::Color32, amount: u8) -> egui::Color32 {
    egui::Color32::from_rgb(
        color.r().saturating_add(amount),
        color.g().saturating_add(amount),
        color.b().saturating_add(amount),
    )
}

pub fn term_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let label = format!(" {text} ");
    let font = ui.style().text_styles.get(&egui::TextStyle::Monospace)
        .cloned()
        .unwrap_or_else(|| egui::FontId::monospace(13.0));
    let fg = ui.visuals().widgets.noninteractive.fg_stroke.color;
    let base = ui.visuals().panel_fill;
    let galley = ui.painter().layout_no_wrap(label, font, fg);
    let (rect, response) = ui.allocate_exact_size(galley.size(), egui::Sense::click());
    let bg = if response.hovered() { lighten(base, 50) } else { lighten(base, 30) };
    ui.painter().rect_filled(rect, 0.0, bg);
    ui.painter().galley(rect.min, galley, fg);
    response
}

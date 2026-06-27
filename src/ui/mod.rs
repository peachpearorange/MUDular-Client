pub mod editor;
pub mod gauges;
pub mod input;
pub mod layout;
pub mod panes;
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

/// Hue-stable button/panel shading that adapts to both dark and light themes.
/// Blends the background toward the foreground, so on dark themes the button
/// lifts (fg is light) and on light themes it depresses (fg is dark). Because
/// this is a real interpolation, channels move together and never drift in hue
/// the way per-channel `saturating_add` or `gamma_multiply(>1)` can.
fn panel_shade(bg: egui::Color32, fg: egui::Color32, t: f32) -> egui::Color32 {
  bg.lerp_to_gamma(fg, t)
}

/// Normal button fill derived from the panel background + foreground.
pub fn panel_button_bg(base: egui::Color32, fg: egui::Color32, hovered: bool) -> egui::Color32 {
  panel_shade(base, fg, if hovered { 0.16 } else { 0.08 })
}

pub fn term_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
  let label = format!(" {text} ");
  let font = ui
    .style()
    .text_styles
    .get(&egui::TextStyle::Monospace)
    .cloned()
    .unwrap_or_else(|| egui::FontId::monospace(13.0));
  let fg = ui.visuals().widgets.noninteractive.fg_stroke.color;
  let base = ui.visuals().panel_fill;
  let galley = ui.painter().layout_no_wrap(label, font, fg);
  let (rect, response) = ui.allocate_exact_size(galley.size(), egui::Sense::click());
  let bg = panel_button_bg(base, fg, response.hovered());
  ui.painter().rect_filled(rect, 0.0, bg);
  ui.painter().galley(rect.min, galley, fg);
  response
}

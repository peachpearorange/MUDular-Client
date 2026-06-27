#[cfg(not(target_arch = "wasm32"))]
use font_kit::{handle::Handle, properties::Properties, source::SystemSource};

#[cfg(not(target_arch = "wasm32"))]
pub fn load_system_font(name: &str) -> Option<Vec<u8>> {
  SystemSource::new()
    .select_best_match(
      &[font_kit::family_name::FamilyName::Title(name.into())],
      &Properties::new()
    )
    .ok()
    .and_then(font_bytes)
}

#[cfg(target_arch = "wasm32")]
pub fn available_fonts() -> Vec<String> {
  vec!["monospace".into(), "proportional".into()]
}

#[cfg(not(target_arch = "wasm32"))]
pub fn available_fonts() -> Vec<String> {
  let mut fonts = SystemSource::new().all_families().unwrap_or_default();
  fonts.sort();
  fonts.dedup();
  fonts
}

#[cfg(not(target_arch = "wasm32"))]
fn font_bytes(handle: Handle) -> Option<Vec<u8>> {
  match handle {
    Handle::Path { path, .. } => std::fs::read(path).ok(),
    Handle::Memory { bytes, .. } => Some((*bytes).clone())
  }
}

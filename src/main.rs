#[cfg(desktop)]
use std::io::Write;

#[cfg(desktop)]
struct DualWriter {
  stderr: std::io::Stderr,
  file: std::sync::Mutex<std::fs::File>
}

#[cfg(desktop)]
impl Write for DualWriter {
  fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    let _ = self.stderr.write(buf);
    let _ = self.file.lock().unwrap().write(buf);
    Ok(buf.len())
  }
  fn flush(&mut self) -> std::io::Result<()> {
    let _ = self.stderr.flush();
    let _ = self.file.lock().unwrap().flush();
    Ok(())
  }
}

#[cfg(desktop)]
fn main() -> eframe::Result<()> {
  let log_path = "/tmp/mudular.log";
  let log_file = std::fs::File::create(log_path).expect("Failed to create log file");
  env_logger::Builder::from_env(
    env_logger::Env::default().default_filter_or("warn,mudular=info")
  )
  .target(env_logger::Target::Pipe(Box::new(DualWriter {
    stderr: std::io::stderr(),
    file: std::sync::Mutex::new(log_file)
  })))
  .init();
  eprintln!("Logging to {log_path}");
  let icon = {
    let png = image::load_from_memory_with_format(
      include_bytes!("../mudular.png"),
      image::ImageFormat::Png
    )
    .expect("embedded icon")
    .into_rgba8();
    let (w, h) = (png.width(), png.height());
    eframe::egui::IconData { rgba: png.into_raw(), width: w, height: h }
  };
  let options = eframe::NativeOptions {
    viewport: eframe::egui::ViewportBuilder::default()
      .with_inner_size([1024.0, 768.0])
      .with_min_inner_size([640.0, 480.0])
      .with_icon(std::sync::Arc::new(icon)),
    ..Default::default()
  };

  eframe::run_native(
    "MUDular Client",
    options,
    Box::new(|cc| Ok(Box::new(mudular::app::MudApp::new(cc))))
  )
}

#[cfg(target_arch = "wasm32")]
fn main() {
  wasm_bindgen_futures::spawn_local(async {
    use wasm_bindgen::JsCast;

    let canvas = web_sys::window()
      .expect("window not available")
      .document()
      .expect("document not available")
      .get_element_by_id("the_canvas_id")
      .expect("canvas not found")
      .dyn_into::<web_sys::HtmlCanvasElement>()
      .expect("element is not a canvas");

    let _ = eframe::WebRunner::new()
      .start(
        canvas,
        eframe::WebOptions::default(),
        Box::new(|cc| Ok(Box::new(mudular::app::MudApp::new(cc))))
      )
      .await;
  });
}

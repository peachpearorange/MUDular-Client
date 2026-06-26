mod ansi;
mod app;
mod buffer;
#[cfg(not(target_arch = "wasm32"))]
mod connection;
#[cfg(not(target_arch = "wasm32"))]
mod probe;
mod profile;
mod protocol;
mod scripting;
mod telnet;
mod themes;
mod ui;
#[cfg(target_arch = "wasm32")]
mod web_connection;

#[cfg(not(target_arch = "wasm32"))]
struct DualWriter {
  stderr: std::io::Stderr,
  file: std::sync::Mutex<std::fs::File>
}

#[cfg(not(target_arch = "wasm32"))]
impl std::io::Write for DualWriter {
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

#[cfg(not(target_arch = "wasm32"))]
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
  let options = eframe::NativeOptions {
    viewport: eframe::egui::ViewportBuilder::default()
      .with_inner_size([1024.0, 768.0])
      .with_min_inner_size([640.0, 480.0]),
    ..Default::default()
  };

  eframe::run_native(
    "MUDular Client",
    options,
    Box::new(|cc| Ok(Box::new(app::MudApp::new(cc))))
  )
}

#[cfg(target_arch = "wasm32")]
fn main() {
  let web_options = eframe::WebOptions::default();
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
      .start(canvas, web_options, Box::new(|cc| Ok(Box::new(app::MudApp::new(cc)))))
      .await;
  });
}

#[cfg(test)]
mod tests {
  use crate::scripting::ScriptEngine;

  #[test]
  fn test_nukefire_profile_loads() {
    let code = include_str!("../profiles/nukefire/init.scm");
    let mut engine = ScriptEngine::new().expect("engine creation failed");
    engine.load_script(code).expect("nukefire script failed to load");

    let st = engine.state.lock().unwrap();
    assert!(st.panes.contains_key("main"));
    assert!(st.panes.contains_key("map"));
    assert_eq!(st.profile_name.as_deref(), Some("NukeFire"));
    assert_eq!(st.profile_host.as_deref(), Some("tdome.nukefire.org"));
    assert_eq!(st.profile_port, Some(4000));
    assert_eq!(st.profile_tls, Some(false));
    assert!(st.gauges.len() >= 3);
    let health = st.gauges.iter().find(|g| g.name == "health").unwrap();
    let mana = st.gauges.iter().find(|g| g.name == "mana").unwrap();
    let moves = st.gauges.iter().find(|g| g.name == "moves").unwrap();
    assert_eq!(health.color, "green");
    assert_eq!(mana.color, "cyan");
    assert_eq!(moves.color, "blue");
  }

  #[test]
  fn test_nukefire_msdp_updates_status_and_gauges() {
    let code = include_str!("../profiles/nukefire/init.scm");
    let mut engine = ScriptEngine::new().expect("engine creation failed");
    engine.load_script(code).expect("nukefire script failed to load");

    engine.handle_msdp(&serde_json::json!({
        "ROOM_NAME": "\u{1b}[1;32mThe Overlook Catwalk\u{1b}[0;00m",
        "AREA_NAME": "NukeFire",
        "ROOM_EXITS": { "north": "123" },
        "HEALTH": "42",
        "HEALTH_MAX": "100",
        "MANA": "17",
        "MANA_MAX": "50",
        "MOVEMENT": "88",
        "MOVEMENT_MAX": "90",
        "LEVEL": "12",
        "EXPERIENCE_TNL": "345",
    }));

    let st = engine.state.lock().unwrap();
    let status_text: String =
      st.status_line.spans.iter().map(|span| span.text.as_str()).collect();
    assert!(status_text.contains("The Overlook Catwalk"));
    assert!(!status_text.contains("\u{1b}"));
    assert!(st.status_line.spans.iter().any(|span| span.style.fg.is_some()));

    let health = st.gauges.iter().find(|g| g.name == "health").unwrap();
    let mana = st.gauges.iter().find(|g| g.name == "mana").unwrap();
    let moves = st.gauges.iter().find(|g| g.name == "moves").unwrap();
    assert_eq!(
      (health.current as i64, health.max as i64, health.color.as_str()),
      (42, 100, "green")
    );
    assert_eq!(
      (mana.current as i64, mana.max as i64, mana.color.as_str()),
      (17, 50, "cyan")
    );
    assert_eq!(
      (moves.current as i64, moves.max as i64, moves.color.as_str()),
      (88, 90, "blue")
    );
  }

  #[test]
  fn test_pane_print_restores_escaped_ansi() {
    let mut engine = ScriptEngine::new().expect("engine creation failed");
    engine
      .load_script(r#"(pane-print "main" "\u{1b}[32m> who\u{1b}[0m")"#)
      .expect("script failed to load");

    let st = engine.state.lock().unwrap();
    let line = st.panes.get("main").unwrap().lines.last().unwrap();
    let text: String = line.spans.iter().map(|span| span.text.as_str()).collect();
    assert_eq!(text, "> who");
    assert!(line.spans.iter().any(|span| span.style.fg.is_some()));
  }

  #[test]
  fn test_nukefire_on_line() {
    let code = include_str!("../profiles/nukefire/init.scm");
    let mut engine = ScriptEngine::new().expect("engine creation failed");
    engine.load_script(code).expect("script load failed");

    // Normal text should pass through
    assert!(engine.handle_line("Hello world"));

    // Prompt echo should be suppressed
    assert!(!engine.handle_line("> n"));

    // Map grid should be captured
    assert!(!engine.handle_line("  |  @  |  "));
  }

  #[test]
  fn test_generated_templates_load() {
    let templates = crate::profile::Profile::templates();
    for template in &templates {
      let mut engine = ScriptEngine::new().expect("engine creation failed");
      engine
        .load_script(&template.script_code)
        .unwrap_or_else(|e| panic!("template '{}' failed to load: {e}", template.name));
      let st = engine.state.lock().unwrap();
      assert!(
        st.panes.contains_key("main"),
        "template '{}' missing main pane",
        template.name
      );
      assert!(
        st.ansi_palette.is_some(),
        "template '{}' missing palette (load_theme failed)",
        template.name
      );
    }
  }

  #[test]
  fn test_nukefire_keymaps() {
    let code = include_str!("../profiles/nukefire/init.scm");
    let mut engine = ScriptEngine::new().expect("engine creation failed");
    engine.load_script(code).expect("script load failed");

    let st = engine.state.lock().unwrap();
    assert!(st.keymaps.len() >= 6);
    assert!(st.keep_input);
    let w_map = st.keymaps.iter().find(|km| km.combo.key == "w").unwrap();
    assert!(w_map.combo.alt);
    assert_eq!(w_map.command, "n");
  }
}

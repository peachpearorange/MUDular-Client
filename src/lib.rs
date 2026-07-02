pub mod ansi;
pub mod app;
pub mod buffer;
#[cfg(not(target_arch = "wasm32"))]
pub mod connection;
#[cfg(desktop)]
pub mod discord_rpc;
pub mod fonts;
#[cfg(not(target_arch = "wasm32"))]
pub mod probe;
pub mod profile;
pub mod protocol;
pub mod scripting;
pub mod telnet;
pub mod themes;
pub mod ui;
#[cfg(target_arch = "wasm32")]
pub mod web_connection;

#[cfg(target_os = "android")]
pub mod android_util {
  use std::sync::OnceLock;
  use std::sync::atomic::{AtomicI32, Ordering};

  struct Ptrs(*mut std::ffi::c_void, *mut std::ffi::c_void);
  unsafe impl Send for Ptrs {}
  unsafe impl Sync for Ptrs {}

  static ANDROID_PTRS: OnceLock<Ptrs> = OnceLock::new();
  static ANDROID_APP: OnceLock<android_activity::AndroidApp> = OnceLock::new();
  static KEYBOARD_HEIGHT: AtomicI32 = AtomicI32::new(0);

  pub fn init(app: &android_activity::AndroidApp) {
    ANDROID_PTRS.get_or_init(|| Ptrs(app.vm_as_ptr(), app.activity_as_ptr()));
    ANDROID_APP.get_or_init(|| app.clone());
  }

  pub fn keep_keyboard_visible() {
    if let Some(app) = ANDROID_APP.get() {
      app.show_soft_input(false);
    }
  }

  pub fn enter_immersive() {
    let Some(Ptrs(vm_ptr, activity_ptr)) = ANDROID_PTRS.get() else { return };
    let (vm_ptr, activity_ptr) = (*vm_ptr, *activity_ptr);
    use jni::objects::JValue;
    unsafe {
      let vm = jni::JavaVM::from_raw(vm_ptr.cast()).unwrap();
      let activity = jni::objects::JObject::from_raw(activity_ptr.cast());
      {
        let mut env = vm.attach_current_thread().unwrap();
        let class_loader = env
          .call_method(&activity, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])
          .unwrap()
          .l()
          .unwrap();
        let class_name = env.new_string("com.mudular.client.ImmersiveHelper").unwrap();
        let helper_class = env
          .call_method(
            &class_loader,
            "loadClass",
            "(Ljava/lang/String;)Ljava/lang/Class;",
            &[JValue::Object(&class_name.into())],
          )
          .unwrap()
          .l()
          .unwrap();
        let method_name = env.new_string("enterImmersive").unwrap();
        let activity_class = env.find_class("android/app/Activity").unwrap();
        let param_types = env
          .new_object_array(1, "java/lang/Class", activity_class)
          .unwrap();
        let method = env
          .call_method(
            &helper_class,
            "getMethod",
            "(Ljava/lang/String;[Ljava/lang/Class;)Ljava/lang/reflect/Method;",
            &[JValue::Object(&method_name.into()), JValue::Object(&param_types.into())],
          )
          .unwrap()
          .l()
          .unwrap();
        let args = env
          .new_object_array(1, "java/lang/Object", &activity)
          .unwrap();
        env
          .call_method(
            &method,
            "invoke",
            "(Ljava/lang/Object;[Ljava/lang/Object;)Ljava/lang/Object;",
            &[JValue::Object(&jni::objects::JObject::null()), JValue::Object(&args.into())],
          )
          .unwrap();
      }
      std::mem::forget(vm);
    }
  }

  pub fn poll_keyboard_height() {
    let Some(Ptrs(vm_ptr, activity_ptr)) = ANDROID_PTRS.get() else { return };
    let (vm_ptr, activity_ptr) = (*vm_ptr, *activity_ptr);
    unsafe {
      let vm = jni::JavaVM::from_raw(vm_ptr.cast()).unwrap();
      let activity = jni::objects::JObject::from_raw(activity_ptr.cast());
      let height = {
        let mut env = vm.attach_current_thread().unwrap();
        let window = env
          .call_method(activity, "getWindow", "()Landroid/view/Window;", &[])
          .unwrap().l().unwrap();
        let decor = env
          .call_method(window, "getDecorView", "()Landroid/view/View;", &[])
          .unwrap().l().unwrap();
        let root_height = env
          .call_method(&decor, "getHeight", "()I", &[])
          .unwrap().i().unwrap();

        let rect_class = env.find_class("android/graphics/Rect").unwrap();
        let rect = env.new_object(rect_class, "()V", &[]).unwrap();
        env.call_method(&decor, "getWindowVisibleDisplayFrame",
          "(Landroid/graphics/Rect;)V",
          &[jni::objects::JValue::Object(&rect)]
        ).unwrap();
        let visible_bottom = env
          .get_field(&rect, "bottom", "I")
          .unwrap().i().unwrap();

        root_height - visible_bottom
      };
      KEYBOARD_HEIGHT.store(height.max(0), Ordering::Relaxed);
      std::mem::forget(vm);
    }
  }

  pub fn keyboard_height() -> f32 {
    KEYBOARD_HEIGHT.load(Ordering::Relaxed) as f32
  }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn web_main() {
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
      .start(canvas, eframe::WebOptions::default(), Box::new(|cc| Ok(Box::new(app::MudApp::new(cc)))))
      .await;
  });
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(android_app: android_activity::AndroidApp) {
  android_logger::init_once(
    android_logger::Config::default().with_max_level(log::LevelFilter::Info)
  );

  if let Some(data_dir) = android_app.internal_data_path() {
    let steel_home = data_dir.join("steel");
    std::fs::create_dir_all(&steel_home).ok();
    unsafe {
      std::env::set_var("STEEL_HOME", &steel_home);
      std::env::set_var("HOME", &data_dir);
    }
  }

  android_util::init(&android_app);
  android_util::enter_immersive();

  let options = eframe::NativeOptions {
    android_app: Some(android_app),
    ..Default::default()
  };

  if let Err(e) = eframe::run_native(
    "MUDular Client",
    options,
    Box::new(|cc| Ok(Box::new(app::MudApp::new(cc))))
  ) {
    log::error!("eframe: {e}");
  }

  std::process::exit(0);
}

#[cfg(test)]
mod tests {
  use crate::scripting::ScriptEngine;

  fn nukefire_template_code() -> String {
    crate::profile::Profile::templates()
      .into_iter()
      .find(|p| p.name == "NukeFire")
      .expect("NukeFire template exists")
      .script_code
  }

  #[test]
  fn test_nukefire_profile_loads() {
    let code = nukefire_template_code();
    let mut engine = ScriptEngine::new().expect("engine creation failed");
    engine.load_script(&code).expect("nukefire script failed to load");

    let st = engine.state.lock().unwrap();
    assert!(st.panes.contains_key("main"));
    assert!(st.panes.contains_key("map"));
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
    let code = nukefire_template_code();
    let mut engine = ScriptEngine::new().expect("engine creation failed");
    engine.load_script(&code).expect("nukefire script failed to load");

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
      .load_script(r#"(mud/pane-print "main" "\u{1b}[32m> who\u{1b}[0m")"#)
      .expect("script failed to load");

    let st = engine.state.lock().unwrap();
    let line = st.panes.get("main").unwrap().lines.last().unwrap();
    let text: String = line.spans.iter().map(|span| span.text.as_str()).collect();
    assert_eq!(text, "> who");
    assert!(line.spans.iter().any(|span| span.style.fg.is_some()));
  }

  #[test]
  fn test_nukefire_on_line() {
    let code = nukefire_template_code();
    let mut engine = ScriptEngine::new().expect("engine creation failed");
    engine.load_script(&code).expect("script load failed");

    assert!(engine.handle_line("Hello world"));
    assert!(!engine.handle_line("> n"));
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
  fn test_default_hooks_dont_error() {
    let templates = crate::profile::Profile::templates();
    let dune = templates.iter().find(|p| p.name == "Dune").expect("Dune template");
    let mut engine = ScriptEngine::new().expect("engine creation failed");
    engine.load_script(&dune.script_code).expect("script load failed");
    engine.handle_input_hook("hello");
    engine.handle_gmcp("Core.Hello", &serde_json::Value::Null);
    engine.handle_msdp(&serde_json::Value::Null);
    engine.handle_line("a line");
  }

  #[test]
  fn test_nukefire_keymaps() {
    let code = nukefire_template_code();
    let mut engine = ScriptEngine::new().expect("engine creation failed");
    engine.load_script(&code).expect("script load failed");

    assert!(engine.keymaps().len() >= 6);
    assert!(engine.state.lock().unwrap().keep_input);
    let w_map = engine.keymaps().iter().find(|km| km.combo.key == "w").unwrap();
    assert!(w_map.combo.alt);
    engine.invoke_keymap(w_map.callback.clone());
    assert!(engine.state.lock().unwrap().outgoing_commands.contains(&"n".to_string()));
  }

  #[test]
  fn test_font_size_keymaps() {
    let code = nukefire_template_code();
    let mut engine = ScriptEngine::new().expect("engine creation failed");
    engine.load_script(&code).expect("script load failed");

    let initial = engine.state.lock().unwrap().font_size;
    let plus_cb = engine
      .keymaps()
      .iter()
      .find(|km| km.combo.key == "plus")
      .unwrap()
      .callback
      .clone();
    engine.invoke_keymap(plus_cb);
    assert_eq!(engine.state.lock().unwrap().font_size, initial + 1.0);

    let minus_cb = engine
      .keymaps()
      .iter()
      .find(|km| km.combo.key == "minus")
      .unwrap()
      .callback
      .clone();
    engine.invoke_keymap(minus_cb.clone());
    engine.invoke_keymap(minus_cb);
    assert_eq!(engine.state.lock().unwrap().font_size, initial - 1.0);

    let text: String = engine
      .state
      .lock()
      .unwrap()
      .panes
      .get("main")
      .unwrap()
      .lines
      .iter()
      .flat_map(|l| l.spans.iter().map(|s| s.text.as_str()))
      .collect();
    assert!(text.contains("[Font size:"));
  }

  #[test]
  fn test_keymap_is_idempotent() {
    let mut engine = ScriptEngine::new().expect("engine creation failed");
    engine.eval_input(r#"(mud/keymap "alt+x" (lambda () (mud/send "x")))"#);
    engine.eval_input(r#"(mud/keymap "alt+x" (lambda () (mud/send "y")))"#);

    let x_maps: Vec<_> = engine
      .keymaps()
      .iter()
      .filter(|km| km.combo.key == "x" && km.combo.alt)
      .collect();
    assert_eq!(x_maps.len(), 1);

    engine.invoke_keymap(x_maps[0].callback.clone());
    let st = engine.state.lock().unwrap();
    assert!(st.outgoing_commands.contains(&"y".to_string()));
    assert!(!st.outgoing_commands.contains(&"x".to_string()));
  }
}

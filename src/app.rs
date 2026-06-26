use {eframe::egui,
     log::{info, trace, warn}};

#[cfg(not(target_arch = "wasm32"))]
use crate::connection::{ConnEvent, Connection};
#[cfg(target_arch = "wasm32")]
use crate::web_connection::{ConnEvent, Connection};
use crate::{profile::{ConnectionMode, Profile},
            scripting::ScriptEngine,
            ui::{editor::{EditorAction, ScriptEditor},
                 gauges::render_gauges,
                 input::InputLine,
                 layout::render_layout,
                 profile_list::{self, ProfileAction, TemplateAction}}};

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Appearance {
  font_name: Option<String>,
  font_size: f32,
  bg_color: Option<[u8; 3]>,
  fg_color: Option<[u8; 3]>
}

impl Appearance {
  #[cfg(not(target_arch = "wasm32"))]
  fn path() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("com", "mudular", "mudular-client")
      .map(|dirs| dirs.config_dir().join("appearance.json"))
  }

  fn load() -> Self {
    #[cfg(not(target_arch = "wasm32"))]
    {
      Self::path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(Self { font_size: 14.0, ..Default::default() })
    }
    #[cfg(target_arch = "wasm32")]
    Self { font_size: 14.0, ..Default::default() }
  }

  fn save(&self) {
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(path) = Self::path() {
      if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
      }
      let _ = std::fs::write(path, serde_json::to_string(self).unwrap_or_default());
    }
  }
}

#[cfg(not(target_arch = "wasm32"))]
fn find_system_font(name: &str) -> Option<std::path::PathBuf> {
  let normalized = name.replace(' ', "");
  let candidates = [
    format!("{normalized}-Regular.ttf"),
    format!("{normalized}-Regular.otf"),
    format!("{normalized}.ttf"),
    format!("{normalized}.otf")
  ];
  let mut search_dirs = vec![
    std::path::PathBuf::from("/usr/share/fonts"),
    std::path::PathBuf::from("/usr/local/share/fonts"),
  ];
  if let Ok(home) = std::env::var("HOME") {
    search_dirs.push(std::path::PathBuf::from(format!("{home}/.local/share/fonts")));
  }
  for dir in &search_dirs {
    for candidate in &candidates {
      if let Some(path) = find_file_recursive(dir, candidate) {
        return Some(path);
      }
    }
  }
  None
}

#[cfg(not(target_arch = "wasm32"))]
fn find_file_recursive(
  dir: &std::path::Path,
  filename: &str
) -> Option<std::path::PathBuf> {
  let entries = std::fs::read_dir(dir).ok()?;
  for entry in entries.flatten() {
    let path = entry.path();
    if path.is_file()
      && path
        .file_name()
        .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(filename))
    {
      return Some(path);
    }
    if path.is_dir() {
      if let Some(found) = find_file_recursive(&path, filename) {
        return Some(found);
      }
    }
  }
  None
}

fn key_name_to_egui(name: &str) -> Option<egui::Key> {
  egui::Key::from_name(name).or_else(|| match name {
    "0" | "num0" => Some(egui::Key::Num0),
    "1" | "num1" => Some(egui::Key::Num1),
    "2" | "num2" => Some(egui::Key::Num2),
    "3" | "num3" => Some(egui::Key::Num3),
    "4" | "num4" => Some(egui::Key::Num4),
    "5" | "num5" => Some(egui::Key::Num5),
    "6" | "num6" => Some(egui::Key::Num6),
    "7" | "num7" => Some(egui::Key::Num7),
    "8" | "num8" => Some(egui::Key::Num8),
    "9" | "num9" => Some(egui::Key::Num9),
    "pageup" => Some(egui::Key::PageUp),
    "pagedown" => Some(egui::Key::PageDown),
    "home" => Some(egui::Key::Home),
    "end" => Some(egui::Key::End),
    "insert" => Some(egui::Key::Insert),
    "delete" => Some(egui::Key::Delete),
    "escape" | "esc" => Some(egui::Key::Escape),
    "tab" => Some(egui::Key::Tab),
    "space" => Some(egui::Key::Space),
    "enter" | "return" => Some(egui::Key::Enter),
    "backspace" => Some(egui::Key::Backspace),
    _ => None
  })
}

fn apply_appearance(ctx: &egui::Context, appearance: &Appearance) -> Option<String> {
  let mut warning = None;

  #[cfg(not(target_arch = "wasm32"))]
  if let Some(name) = &appearance.font_name {
    if let Some(path) = find_system_font(name) {
      if let Ok(data) = std::fs::read(&path) {
        let mut fonts = egui::FontDefinitions::default();
        fonts
          .font_data
          .insert("custom_mono".into(), egui::FontData::from_owned(data).into());
        fonts
          .families
          .entry(egui::FontFamily::Monospace)
          .or_default()
          .insert(0, "custom_mono".into());
        fonts
          .families
          .entry(egui::FontFamily::Proportional)
          .or_default()
          .insert(0, "custom_mono".into());
        ctx.set_fonts(fonts);
      } else {
        warning = Some(format!(
          "[Font '{name}' was found but could not be loaded; using the default font]"
        ));
      }
    } else {
      warning = Some(format!("[Font '{name}' is not available; using the default font]"));
    }
  }
  #[cfg(target_arch = "wasm32")]
  if let Some(name) = &appearance.font_name {
    warning = Some(format!(
      "[Font '{name}' cannot be loaded from system fonts in the web build; using the default font]"
    ));
  }

  let mut style = (*ctx.global_style()).clone();
  style.visuals = egui::Visuals::dark();
  let font = egui::FontId::monospace(appearance.font_size);
  style.text_styles.insert(egui::TextStyle::Monospace, font.clone());
  style.text_styles.insert(egui::TextStyle::Body, font.clone());
  style.text_styles.insert(egui::TextStyle::Small, font.clone());
  style.text_styles.insert(
    egui::TextStyle::Heading,
    egui::FontId::monospace(appearance.font_size + 2.0)
  );
  style.text_styles.insert(egui::TextStyle::Button, font);

  if let Some([r, g, b]) = appearance.bg_color {
    let bg = egui::Color32::from_rgb(r, g, b);
    style.visuals.panel_fill = bg;
    style.visuals.window_fill = bg;
    style.visuals.extreme_bg_color = bg;
    style.visuals.faint_bg_color = egui::Color32::from_rgba_premultiplied(
      r.saturating_add(10),
      g.saturating_add(10),
      b.saturating_add(10),
      255
    );
  }
  if let Some([r, g, b]) = appearance.fg_color {
    let fg = egui::Color32::from_rgb(r, g, b);
    let stroke = egui::Stroke::new(1.0_f32, fg);
    style.visuals.widgets.noninteractive.fg_stroke = stroke;
    style.visuals.widgets.inactive.fg_stroke = stroke;
    style.visuals.widgets.hovered.fg_stroke = stroke;
    style.visuals.widgets.active.fg_stroke = stroke;
    style.visuals.widgets.open.fg_stroke = stroke;
    style.visuals.widgets.noninteractive.bg_stroke = stroke;
  }

  style.spacing.window_margin = egui::Margin::ZERO;
  ctx.set_global_style(style);

  warning
}

struct Session {
  profile_idx: usize,
  name: String,
  connection: Option<Connection>,
  script_engine: ScriptEngine,
  input: InputLine
}

impl Session {
  fn process_events(&mut self) -> bool {
    let Some(conn) = &self.connection else { return false };
    let events = conn.poll_events();
    let mut disconnected = false;
    for event in events {
      match event {
        ConnEvent::Connected => {
          info!("Connected");
          self.script_engine.append_system_message("[Connected]");
          self.script_engine.handle_connect();
          info!("After on_connect: {} timers", self.script_engine.timer_count());
        }
        ConnEvent::Data(line) => {
          info!("DATA: {}", line.chars().take(120).collect::<String>());
          let show = self.script_engine.handle_line(&line);
          if show {
            self.script_engine.append_to_main(&line);
          }
        }
        #[cfg(not(target_arch = "wasm32"))]
        ConnEvent::GmcpReceived(package, data) => {
          info!("GMCP: {package}");
          self.script_engine.handle_gmcp(&package, &data);
        }
        #[cfg(not(target_arch = "wasm32"))]
        ConnEvent::MsspReceived(mssp_info) => {
          let msg = format!("[MSSP: {} entries received]", mssp_info.len());
          info!("{msg}");
          self.script_engine.append_system_message(&msg);
        }
        #[cfg(not(target_arch = "wasm32"))]
        ConnEvent::MsdpReceived(data) => {
          info!("MSDP: {data}");
          if let Some(vars) = data.get("REPORTABLE_VARIABLES") {
            if let Some(arr) = vars.as_array() {
              let names: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
              self.script_engine.append_system_message(&format!(
                "[MSDP reportable variables: {}]",
                names.join(", ")
              ));
            }
          }
          self.script_engine.handle_msdp(&data);
        }
        ConnEvent::Disconnected(reason) => {
          warn!("Disconnected: {reason}");
          self.script_engine.append_system_message(&format!("[Disconnected: {reason}]"));
          self.script_engine.handle_disconnect();
          self.connection = None;
          disconnected = true;
        }
        ConnEvent::Error(e) => {
          warn!("Connection error: {e}");
          self.script_engine.append_system_message(&format!("[Error: {e}]"));
        }
      }
    }
    disconnected
  }

  fn process_outgoing(&mut self) {
    let Some(conn) = &self.connection else { return };
    for cmd in self.script_engine.drain_commands() {
      info!("Sending command ({} chars)", cmd.len());
      conn.send(&cmd);
    }
    #[cfg(not(target_arch = "wasm32"))]
    for (package, data) in self.script_engine.drain_gmcp() {
      info!("Sending GMCP: {package}");
      conn.send_gmcp(&package, &data);
    }
    #[cfg(target_arch = "wasm32")]
    let _ = self.script_engine.drain_gmcp();
    #[cfg(not(target_arch = "wasm32"))]
    for vars in self.script_engine.drain_msdp_reports() {
      info!("Sending MSDP report for: {vars:?}");
      conn.send_msdp_report(vars);
    }
    #[cfg(target_arch = "wasm32")]
    let _ = self.script_engine.drain_msdp_reports();
    #[cfg(not(target_arch = "wasm32"))]
    for vars in self.script_engine.drain_msdp_sends() {
      info!("Sending MSDP send for: {vars:?}");
      conn.send_msdp_send(vars);
    }
    #[cfg(target_arch = "wasm32")]
    let _ = self.script_engine.drain_msdp_sends();
    #[cfg(not(target_arch = "wasm32"))]
    for what in self.script_engine.drain_msdp_lists() {
      info!("Sending MSDP list: {what}");
      conn.send_msdp_list(what);
    }
    #[cfg(target_arch = "wasm32")]
    let _ = self.script_engine.drain_msdp_lists();
  }
}

pub struct MudApp {
  profiles: Vec<Profile>,
  templates: Vec<Profile>,
  sessions: Vec<Session>,
  active_tab: usize,
  last_active_tab: usize,
  editor: ScriptEditor,
  editor_profile_idx: Option<usize>,
  #[cfg(not(target_arch = "wasm32"))]
  runtime: tokio::runtime::Runtime,
  new_profile_name: String,
  new_profile_host: String,
  new_profile_port: String,
  show_new_profile_dialog: bool,
  show_template_picker: bool,
  rename_idx: Option<usize>,
  rename_name: String,
  loaded_font_name: Option<String>,
  delete_confirm_idx: Option<usize>,
  #[cfg(not(target_arch = "wasm32"))]
  mssp_info: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
  #[cfg(not(target_arch = "wasm32"))]
  mssp_rx: std::sync::mpsc::Receiver<(String, std::collections::HashMap<String, String>)>,
  #[cfg(not(target_arch = "wasm32"))]
  mssp_tx: std::sync::mpsc::Sender<(String, std::collections::HashMap<String, String>)>,
  #[cfg(not(target_arch = "wasm32"))]
  mssp_probed: std::collections::HashSet<String>
}

impl MudApp {
  pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
    #[cfg(not(target_arch = "wasm32"))]
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");

    #[cfg(not(target_arch = "wasm32"))]
    let (mssp_tx, mssp_rx) = std::sync::mpsc::channel();

    cc.egui_ctx.set_visuals(egui::Visuals::dark());
    let appearance = Appearance::load();
    let _ = apply_appearance(&cc.egui_ctx, &appearance);
    let templates = Profile::templates();
    #[cfg(not(target_arch = "wasm32"))]
    let profiles = Profile::load_user();
    #[cfg(target_arch = "wasm32")]
    let profiles = templates
      .iter()
      .filter(|profile| profile.connection_mode == ConnectionMode::WebSocket)
      .cloned()
      .collect();

    Self {
      profiles,
      templates,
      sessions: Vec::new(),
      active_tab: 0,
      last_active_tab: 0,
      editor: ScriptEditor::new(),
      editor_profile_idx: None,
      #[cfg(not(target_arch = "wasm32"))]
      runtime,
      new_profile_name: String::new(),
      new_profile_host: String::new(),
      new_profile_port: "23".into(),
      show_new_profile_dialog: false,
      show_template_picker: false,
      rename_idx: None,
      rename_name: String::new(),
      loaded_font_name: None,
      delete_confirm_idx: None,
      #[cfg(not(target_arch = "wasm32"))]
      mssp_info: std::collections::HashMap::new(),
      #[cfg(not(target_arch = "wasm32"))]
      mssp_rx,
      #[cfg(not(target_arch = "wasm32"))]
      mssp_tx,
      #[cfg(not(target_arch = "wasm32"))]
      mssp_probed: std::collections::HashSet::new()
    }
  }

  #[cfg(not(target_arch = "wasm32"))]
  fn connect_to_profile(&mut self, idx: usize) {
    let profile = &self.profiles[idx];
    info!(
      "Connecting to profile '{}' at {}:{}",
      profile.name, profile.host, profile.port
    );
    let mut engine = ScriptEngine::new().expect("Failed to create script engine");
    engine.state.lock().unwrap().profile_dir =
      profile.path.as_ref().and_then(|p| p.parent()).map(|p| p.to_path_buf());
    if let Err(e) = engine.load_script(&profile.script_code) {
      engine.append_system_message(&format!("[Script error: {e}]"));
    }
    let connection = match profile.connection_mode {
      ConnectionMode::Tcp => Some(Connection::connect(
        profile.host.clone(),
        profile.port,
        profile.tls,
        &self.runtime
      )),
      ConnectionMode::WebSocket => {
        engine.append_system_message(
          "[This profile uses WebSocket connections, which are only supported in the web build]"
        );
        None
      }
    };
    info!(
      "Script loaded: {} triggers, {} aliases, {} timers",
      engine.trigger_count(),
      engine.alias_count(),
      engine.timer_count()
    );

    self.sessions.push(Session {
      profile_idx: idx,
      name: profile.name.clone(),
      connection,
      script_engine: engine,
      input: InputLine::new()
    });
    self.active_tab = self.sessions.len();
  }

  #[cfg(target_arch = "wasm32")]
  fn connect_to_profile(&mut self, idx: usize) {
    let profile = &self.profiles[idx];
    let mut engine = ScriptEngine::new().expect("Failed to create script engine");
    if let Err(e) = engine.load_script(&profile.script_code) {
      engine.append_system_message(&format!("[Script error: {e}]"));
    }

    let connection = match profile.connection_mode {
      ConnectionMode::WebSocket => profile
        .websocket_url
        .as_deref()
        .map(Connection::connect)
        .transpose()
        .unwrap_or_else(|e| {
          engine.append_system_message(&format!("[WebSocket error: {e}]"));
          None
        }),
      ConnectionMode::Tcp => {
        engine.append_system_message(
          "[This profile uses TCP connections, which browsers cannot open directly]"
        );
        None
      }
    };
    if profile.connection_mode == ConnectionMode::WebSocket
      && profile.websocket_url.is_none()
    {
      engine.append_system_message(
        "[This WebSocket profile does not define a WebSocket URL]"
      );
    }

    self.sessions.push(Session {
      profile_idx: idx,
      name: profile.name.clone(),
      connection,
      script_engine: engine,
      input: InputLine::new()
    });
    self.active_tab = self.sessions.len();
  }

  fn disconnect_session(&mut self, session_idx: usize) {
    if let Some(conn) = &self.sessions[session_idx].connection {
      conn.disconnect();
    }
    self.sessions[session_idx].script_engine.handle_disconnect();
    self.sessions.remove(session_idx);
    if self.active_tab > 0 {
      let active_si = self.active_tab - 1;
      if active_si == session_idx {
        self.active_tab = 0;
      } else if active_si > session_idx {
        self.active_tab -= 1;
      }
    }
  }

  fn process_keymaps(&mut self, ctx: &egui::Context) {
    let si = match self.active_tab.checked_sub(1) {
      Some(i) if i < self.sessions.len() => i,
      _ => return
    };
    let session = &mut self.sessions[si];
    let keymaps = session.script_engine.state.lock().unwrap().keymaps.clone();
    let mut matched = false;
    let mut commands = Vec::new();
    ctx.input_mut(|i| {
      for km in &keymaps {
        if let Some(key) = key_name_to_egui(&km.combo.key) {
          if i.key_pressed(key)
            && i.modifiers.alt == km.combo.alt
            && i.modifiers.ctrl == km.combo.ctrl
            && i.modifiers.shift == km.combo.shift
          {
            commands.push(km.command.clone());
            i.consume_key(
              egui::Modifiers {
                alt: km.combo.alt,
                ctrl: km.combo.ctrl,
                shift: km.combo.shift,
                ..Default::default()
              },
              key
            );
            matched = true;
          }
        }
      }
    });
    for command in commands {
      let parts: Vec<&str> = command.splitn(2, ' ').collect();
      match parts[0] {
        "scroll_up" => {
          let lines = parts.get(1).and_then(|s| s.parse::<f32>().ok()).unwrap_or(10.0);
          let mut st = session.script_engine.state.lock().unwrap();
          if let Some(buf) = st.panes.get_mut("main") {
            buf.scroll_delta_lines += lines;
            buf.auto_scroll = false;
          }
        }
        "scroll_down" => {
          let lines = parts.get(1).and_then(|s| s.parse::<f32>().ok()).unwrap_or(10.0);
          let mut st = session.script_engine.state.lock().unwrap();
          if let Some(buf) = st.panes.get_mut("main") {
            buf.scroll_delta_lines -= lines;
          }
        }
        _ =>
        {
          #[cfg(not(target_arch = "wasm32"))]
          if let Some(conn) = &session.connection {
            conn.send(&command);
          }
        }
      }
    }
    session.input.keymap_matched = matched;
  }

  fn apply_theme(&mut self, ctx: &egui::Context) {
    let appearance = {
      let si = match self.active_tab.checked_sub(1) {
        Some(i) => i,
        None => return
      };
      let session = match self.sessions.get(si) {
        Some(s) => s,
        None => return
      };
      let mut st = session.script_engine.state.lock().unwrap();
      if !st.theme_dirty {
        return;
      }
      st.theme_dirty = false;
      Appearance {
        font_name: st.font_name.clone(),
        font_size: st.font_size,
        bg_color: st.bg_color,
        fg_color: st.fg_color
      }
    };

    if appearance.font_name != self.loaded_font_name {
      self.loaded_font_name = appearance.font_name.clone();
    }

    if let Some(warning) = apply_appearance(ctx, &appearance)
      && let Some(si) = self.active_tab.checked_sub(1)
      && let Some(session) = self.sessions.get_mut(si)
    {
      session.script_engine.append_system_message(&warning);
    }
    appearance.save();
  }

  fn render_session_content(&mut self, ui: &mut egui::Ui, session_idx: usize) {
    let editor_visible = self.editor.visible;
    let session = &mut self.sessions[session_idx];

    let (gauges, status_line) = {
      let st = session.script_engine.state.lock().unwrap();
      (st.gauges.clone(), st.status_line.clone())
    };

    let font_size = ui
      .style()
      .text_styles
      .get(&egui::TextStyle::Monospace)
      .map(|f| f.size)
      .unwrap_or(13.0);
    let spacing = ui.spacing().item_spacing.y;
    let available = ui.available_height();
    let line_height = font_size + 6.0;
    let gauge_height = if gauges.is_empty() { 0.0 } else { line_height + spacing };
    let separator_height = 4.0;
    let status_height = if status_line.is_empty() { 0.0 } else { line_height + spacing };
    let bottom_height =
      line_height + gauge_height + status_height + separator_height + spacing;
    let pane_height = (available - bottom_height).max(100.0);
    trace!(
      "Layout: available={available} bottom={bottom_height} pane_height={pane_height}"
    );

    ui.allocate_ui(egui::vec2(ui.available_width(), pane_height), |ui| {
      let mut st = session.script_engine.state.lock().unwrap();
      let layout = st.layout.clone();
      render_layout(ui, &layout, &mut st.panes);
    });

    ui.separator();
    if !status_line.is_empty() {
      let font_id = ui
        .style()
        .text_styles
        .get(&egui::TextStyle::Monospace)
        .cloned()
        .unwrap_or_else(|| egui::FontId::monospace(13.0));
      let default_color = ui.visuals().text_color();
      let available_width = ui.available_width();
      crate::ui::panes::render_styled_line(
        ui,
        &status_line,
        available_width,
        &font_id,
        default_color
      );
    }
    render_gauges(ui, &gauges);

    let keep_input = session.script_engine.state.lock().unwrap().keep_input;
    let connected = session.connection.is_some();
    session.input.render(ui, connected, keep_input, editor_visible);
  }
}

impl eframe::App for MudApp {
  fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
    self
      .active_tab
      .checked_sub(1)
      .and_then(|i| self.sessions.get(i))
      .and_then(|s| s.script_engine.state.lock().ok())
      .and_then(|st| st.bg_color)
      .map(|[r, g, b]| egui::Color32::from_rgb(r, g, b))
      .unwrap_or(_visuals.panel_fill)
      .to_normalized_gamma_f32()
  }

  fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    ctx.request_repaint_after(std::time::Duration::from_millis(50));

    #[cfg(not(target_arch = "wasm32"))]
    {
      while let Ok((key, info)) = self.mssp_rx.try_recv() {
        self.mssp_info.insert(key, info);
      }
      if self.active_tab == 0 {
        let to_probe: Vec<(String, u16)> = self
          .profiles
          .iter()
          .filter(|p| !self.mssp_probed.contains(&format!("{}:{}", p.host, p.port)))
          .map(|p| (p.host.clone(), p.port))
          .collect();
        for (host, port) in to_probe {
          let key = format!("{host}:{port}");
          self.mssp_probed.insert(key);
          let tx = self.mssp_tx.clone();
          self.runtime.spawn(async move {
            if let Some(info) = crate::probe::probe_mssp(&host, port).await {
              let _ = tx.send((format!("{host}:{port}"), info));
            }
          });
        }
      }
    }

    let mut disconnected = Vec::new();
    for (i, session) in self.sessions.iter_mut().enumerate() {
      if session.process_events() {
        disconnected.push(i);
      }
      session.script_engine.tick_timers();
      session.process_outgoing();
    }
    for i in disconnected.into_iter().rev() {
      self.sessions.remove(i);
      if self.active_tab > 0 {
        let active_si = self.active_tab - 1;
        if active_si == i {
          self.active_tab = 0;
        } else if active_si > i {
          self.active_tab -= 1;
        }
      }
    }

    let scroll_line_height = self
      .active_tab
      .checked_sub(1)
      .and_then(|i| self.sessions.get(i))
      .map(|s| {
        let st = s.script_engine.state.lock().unwrap();
        st.scroll_lines * (st.font_size + 4.0)
      })
      .unwrap_or(150.0);
    ctx.options_mut(|opts| {
      opts.input_options.line_scroll_speed = scroll_line_height;
    });

    self.process_keymaps(ctx);

    if self.active_tab != self.last_active_tab {
      self.last_active_tab = self.active_tab;
      if let Some(si) = self.active_tab.checked_sub(1) {
        if let Some(session) = self.sessions.get(si) {
          session.script_engine.state.lock().unwrap().theme_dirty = true;
        }
      }
    }
    self.apply_theme(ctx);

    if let Some(si) = self.active_tab.checked_sub(1) {
      if let Some(session) = self.sessions.get_mut(si) {
        if let Some(cmd) = session.input.take_submitted() {
          info!("Input submitted: {cmd:?}");
          session.script_engine.handle_input_hook(&cmd);
          let should_send = session.script_engine.handle_input(&cmd);
          info!("Alias check: should_send={should_send}");
          if should_send {
            if let Some(conn) = &session.connection {
              conn.send(&cmd);
            }
          }
          session.process_outgoing();
        }
      }
    }

    let editor_action = self.editor.render(ctx);
    match editor_action {
      EditorAction::SaveAndReload(code) => {
        if let Some(profile_idx) = self.editor_profile_idx {
          self.profiles[profile_idx].script_code = code.clone();
          let _ = self.profiles[profile_idx].save();
          for session in &mut self.sessions {
            if session.profile_idx == profile_idx {
              if let Err(e) = session.script_engine.load_script(&code) {
                session
                  .script_engine
                  .append_system_message(&format!("[Script reload error: {e}]"));
              } else {
                session.script_engine.append_system_message("[Script reloaded]");
              }
            }
          }
        }
      }
      EditorAction::None => {}
    }
  }

  fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
    let mut want_disconnect = false;
    let mut want_edit = false;

    ui.spacing_mut().item_spacing.y = 0.0;
    ui.horizontal(|ui| {
      if self.active_tab == 0 {
        ui.add(
          egui::Label::new(egui::RichText::new(" Menu ").strong()).selectable(false)
        );
      } else if crate::ui::term_button(ui, "Menu").clicked() {
        self.active_tab = 0;
      }

      for (i, session) in self.sessions.iter().enumerate() {
        if self.active_tab == i + 1 {
          ui.add(
            egui::Label::new(egui::RichText::new(format!(" {} ", session.name)).strong())
              .selectable(false)
          );
        } else if crate::ui::term_button(ui, &session.name).clicked() {
          self.active_tab = i + 1;
        }
      }

      if self.active_tab > 0 {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
          if crate::ui::term_button(ui, "Disconnect").clicked() {
            want_disconnect = true;
          }
          if crate::ui::term_button(ui, "Edit Script").clicked() {
            want_edit = true;
          }
        });
      }
    });
    ui.separator();

    if self.active_tab == 0 {
      #[cfg(not(target_arch = "wasm32"))]
      let mssp = &self.mssp_info;
      #[cfg(target_arch = "wasm32")]
      let mssp = &std::collections::HashMap::new();
      let action = profile_list::render_profile_list(ui, &self.profiles, mssp);
      match action {
        #[cfg(not(target_arch = "wasm32"))]
        ProfileAction::Connect(idx) => self.connect_to_profile(idx),
        #[cfg(target_arch = "wasm32")]
        ProfileAction::Connect(idx) => self.connect_to_profile(idx),
        ProfileAction::EditScript(idx) => {
          self.editor_profile_idx = Some(idx);
          self.editor.open(&self.profiles[idx].script_code);
        }
        ProfileAction::ShowTemplatePicker => {
          self.show_template_picker = true;
        }
        ProfileAction::DeleteProfile(idx) => {
          self.delete_confirm_idx = Some(idx);
        }
        ProfileAction::RenameProfile(idx) => {
          self.rename_name = self.profiles[idx].name.clone();
          self.rename_idx = Some(idx);
        }
        ProfileAction::None => {}
      }

      if self.show_template_picker {
        let action = profile_list::render_template_picker(
          ui.ctx(),
          &self.templates,
          &mut self.show_template_picker
        );
        match action {
          TemplateAction::CreateFromTemplate(idx) => {
            self.show_template_picker = false;
            self.create_from_template(idx);
          }
          TemplateAction::CustomProfile => {
            self.show_template_picker = false;
            self.show_new_profile_dialog = true;
          }
          TemplateAction::Cancel => {
            self.show_template_picker = false;
          }
          TemplateAction::None => {}
        }
      }

      if self.show_new_profile_dialog {
        self.render_new_profile_dialog(ui.ctx());
      }
      self.render_rename_dialog(ui.ctx());
      self.render_delete_confirm_dialog(ui.ctx());
    } else {
      let si = self.active_tab - 1;
      if si < self.sessions.len() {
        self.render_session_content(ui, si);
      }
    }

    if want_edit && self.active_tab > 0 {
      let si = self.active_tab - 1;
      if si < self.sessions.len() {
        let profile_idx = self.sessions[si].profile_idx;
        self.editor_profile_idx = Some(profile_idx);
        self.editor.open(&self.profiles[profile_idx].script_code);
      }
    }
    if want_disconnect && self.active_tab > 0 {
      let si = self.active_tab - 1;
      if si < self.sessions.len() {
        self.disconnect_session(si);
      }
    }
  }
}

impl MudApp {
  fn create_from_template(&mut self, template_idx: usize) {
    let template = &self.templates[template_idx];
    let name = Profile::unique_name(&template.name, &self.profiles);
    let script_code = template.script_code.replacen(
      &format!("  'name \"{}\"", template.name),
      &format!("  'name \"{name}\""),
      1
    );
    let mut profile = Profile {
      name,
      connection_mode: template.connection_mode,
      host: template.host.clone(),
      port: template.port,
      tls: template.tls,
      websocket_url: template.websocket_url.clone(),
      script_code,
      path: None,
      is_preset: false
    };
    let _ = profile.save();
    self.profiles.push(profile);
  }

  fn render_new_profile_dialog(&mut self, ctx: &egui::Context) {
    let mut open = self.show_new_profile_dialog;
    egui::Window::new("New Profile")
      .collapsible(false)
      .resizable(false)
      .open(&mut open)
      .show(ctx, |ui| {
        ui.horizontal(|ui| {
          ui.label("Name:");
          ui.text_edit_singleline(&mut self.new_profile_name);
        });
        ui.horizontal(|ui| {
          ui.label("Host:");
          ui.text_edit_singleline(&mut self.new_profile_host);
        });
        ui.horizontal(|ui| {
          ui.label("Port:");
          ui.text_edit_singleline(&mut self.new_profile_port);
        });
        ui.add_space(8.0);
        if crate::ui::term_button(ui, "Create").clicked()
          && !self.new_profile_name.is_empty()
        {
          let port = self.new_profile_port.parse().unwrap_or(23);
          let code = format!(
            r#"(profile
  'name "{name}"
  'connection-mode 'tcp
  'host "{host}"
  'port {port}
  'tls #f)

(load-theme "Onenord")

(keymap "PageUp" "scroll_up 20")
(keymap "PageDown" "scroll_down 20")

(pane "main")

(define (on-connect)
  (pane-print "main" "[Connected to {name}]"))

(define (on-disconnect)
  (pane-print "main" "[Disconnected from {name}]"))

(define (on-line line) #t)
"#,
            name = self.new_profile_name,
            host = self.new_profile_host,
            port = port,
          );
          let mut profile = Profile {
            name: self.new_profile_name.clone(),
            connection_mode: ConnectionMode::Tcp,
            host: self.new_profile_host.clone(),
            port,
            tls: false,
            websocket_url: None,
            script_code: code,
            path: None,
            is_preset: false
          };
          let _ = profile.save();
          self.profiles.push(profile);
          self.new_profile_name.clear();
          self.new_profile_host.clear();
          self.new_profile_port = "23".into();
          self.show_new_profile_dialog = false;
        }
      });
    self.show_new_profile_dialog = open;
  }

  fn render_rename_dialog(&mut self, ctx: &egui::Context) {
    let Some(idx) = self.rename_idx else { return };
    let mut open = true;
    egui::Window::new("Rename Profile")
      .collapsible(false)
      .resizable(false)
      .open(&mut open)
      .show(ctx, |ui| {
        ui.horizontal(|ui| {
          ui.label("Name:");
          ui.text_edit_singleline(&mut self.rename_name);
        });
        ui.add_space(8.0);
        if crate::ui::term_button(ui, "Rename").clicked() && !self.rename_name.is_empty()
        {
          let _ = self.profiles[idx].rename(&self.rename_name);
          self.rename_idx = None;
        }
      });
    if !open {
      self.rename_idx = None;
    }
  }

  fn render_delete_confirm_dialog(&mut self, ctx: &egui::Context) {
    if let Some(idx) = self.delete_confirm_idx {
      let mut do_delete = false;
      let mut cancel = false;
      let mut open = true;
      let name = self.profiles[idx].name.clone();
      egui::Window::new("Delete Profile")
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
          ui.label(format!("Delete '{name}'?"));
          ui.add_space(8.0);
          ui.horizontal(|ui| {
            if crate::ui::term_button(ui, "Delete").clicked() {
              do_delete = true;
            }
            if crate::ui::term_button(ui, "Cancel").clicked() {
              cancel = true;
            }
          });
        });
      if do_delete {
        let _ = self.profiles[idx].delete();
        self.profiles.remove(idx);
        self.delete_confirm_idx = None;
        #[cfg(not(target_arch = "wasm32"))]
        self.mssp_probed.clear();
      } else if cancel || !open {
        self.delete_confirm_idx = None;
      }
    }
  }
}

pub mod api;

use std::sync::{Arc, Mutex};

use {log::{info, warn},
     steel::{HashMap, gc::Gc, rvals::SteelVal, steel_vm::engine::Engine}};

use eframe::egui::Color32;

use crate::{ansi::parse_ansi,
            buffer::{StyledLine, TextBuffer}};

#[derive(Clone, Debug)]
pub struct Gauge {
  pub name: String,
  pub current: f64,
  pub max: f64,
  pub color: String
}

struct Trigger {
  pattern: String,
  callback: SteelVal
}

struct Alias {
  pattern: String,
  callback: SteelVal
}

struct Timer {
  interval_secs: f64,
  callback: SteelVal,
  last_fired_secs: f64,
  oneshot: bool
}

#[derive(Clone, Debug)]
pub struct LayoutEntry {
  pub pane: String,
  pub weight: f32
}

#[derive(Clone, Debug)]
pub enum LayoutDir {
  Horizontal,
  Vertical
}

#[derive(Clone, Debug)]
pub struct Layout {
  pub direction: LayoutDir,
  pub entries: Vec<LayoutEntry>
}

#[derive(Clone, Debug)]
pub struct KeyCombo {
  pub alt: bool,
  pub ctrl: bool,
  pub shift: bool,
  pub key: String
}

#[derive(Clone, Debug)]
pub struct Keymap {
  pub combo: KeyCombo,
  pub command: String
}

pub struct ScriptState {
  pub panes: std::collections::HashMap<String, TextBuffer>,
  pub gauges: Vec<Gauge>,
  pub layout: Layout,
  pub outgoing_commands: Vec<String>,
  pub outgoing_gmcp: Vec<(String, serde_json::Value)>,
  pub outgoing_msdp_report: Vec<Vec<String>>,
  pub outgoing_msdp_send: Vec<Vec<String>>,
  pub outgoing_msdp_list: Vec<String>,
  pub keymaps: Vec<Keymap>,
  pub keep_input: bool,
  pub font_name: Option<String>,
  pub font_size: f32,
  pub bg_color: Option<[u8; 3]>,
  pub fg_color: Option<[u8; 3]>,
  pub ansi_palette: Option<[Color32; 16]>,
  pub profile_dir: Option<std::path::PathBuf>,
  pub status_line: StyledLine,
  pub theme_dirty: bool,
  pub scroll_lines: f32,
  pub profile_name: Option<String>,
  pub profile_connection_mode: Option<String>,
  pub profile_host: Option<String>,
  pub profile_port: Option<u16>,
  pub profile_tls: Option<bool>,
  pub profile_websocket_url: Option<String>,
  pub profile_websocket_protocol: Option<String>
}

impl ScriptState {
  fn new() -> Self {
    let mut panes = std::collections::HashMap::new();
    panes.insert("main".into(), TextBuffer::new(10000));

    Self {
      panes,
      gauges: Vec::new(),
      layout: Layout {
        direction: LayoutDir::Horizontal,
        entries: vec![LayoutEntry { pane: "main".into(), weight: 1.0 }]
      },
      outgoing_commands: Vec::new(),
      outgoing_gmcp: Vec::new(),
      outgoing_msdp_report: Vec::new(),
      outgoing_msdp_send: Vec::new(),
      outgoing_msdp_list: Vec::new(),
      keymaps: Vec::new(),
      keep_input: false,
      font_name: None,
      font_size: 13.0,
      bg_color: None,
      fg_color: None,
      ansi_palette: None,
      profile_dir: None,
      status_line: StyledLine { spans: Vec::new() },
      theme_dirty: false,
      scroll_lines: 6.0,
      profile_name: None,
      profile_connection_mode: None,
      profile_host: None,
      profile_port: None,
      profile_tls: None,
      profile_websocket_url: None,
      profile_websocket_protocol: None
    }
  }
}

pub struct ScriptEngine {
  engine: Engine,
  pub state: Arc<Mutex<ScriptState>>,
  triggers: Vec<Trigger>,
  aliases: Vec<Alias>,
  timers: Vec<Timer>,
  hooks: std::collections::HashMap<String, SteelVal>
}

impl ScriptEngine {
  pub fn new() -> Result<Self, String> {
    let state = Arc::new(Mutex::new(ScriptState::new()));
    let mut engine = Engine::new();
    api::register_api(&mut engine, state.clone());

    Ok(Self {
      engine,
      state,
      triggers: Vec::new(),
      aliases: Vec::new(),
      timers: Vec::new(),
      hooks: std::collections::HashMap::new()
    })
  }

  pub fn load_script(&mut self, code: &str) -> Result<(), String> {
    self.triggers.clear();
    self.aliases.clear();
    self.timers.clear();

    {
      let mut st = self.state.lock().unwrap();
      let profile_dir = st.profile_dir.take();
      let panes = std::mem::take(&mut st.panes);
      let gauges = std::mem::take(&mut st.gauges);
      *st = ScriptState::new();
      st.profile_dir = profile_dir;
      st.panes = panes;
      st.gauges = gauges;
    }

    self.engine = Engine::new();
    api::register_api(&mut self.engine, self.state.clone());

    self.engine.run(code.to_string()).map_err(|e| format!("{e}"))?;
    self.sync_registrations();
    info!(
      "Script loaded: {} triggers, {} aliases, {} timers",
      self.triggers.len(),
      self.aliases.len(),
      self.timers.len()
    );
    let st = self.state.lock().unwrap();
    info!(
      "Panes: {:?}, Gauges: {}, Layout entries: {}",
      st.panes.keys().collect::<Vec<_>>(),
      st.gauges.len(),
      st.layout.entries.len()
    );

    Ok(())
  }

  fn sync_registrations(&mut self) {
    self.triggers.clear();
    self.aliases.clear();
    self.timers.clear();
    self.hooks.clear();
    if let Ok(SteelVal::HashMapV(hm)) = self.engine.extract_value("*hooks*") {
      for (k, v) in hm.iter() {
        if let SteelVal::StringV(name) = k {
          self.hooks.insert(name.to_string(), v.clone());
        }
      }
    }
    if let Ok(val) = self.engine.extract_value("*triggers*") {
      for item in steel_list_iter(&val) {
        if let (Some(pattern), Some(callback)) = (steel_car(&item), steel_cdr(&item)) {
          if let SteelVal::StringV(s) = pattern {
            self.triggers.push(Trigger { pattern: s.to_string(), callback });
          }
        }
      }
    }
    if let Ok(val) = self.engine.extract_value("*aliases*") {
      for item in steel_list_iter(&val) {
        if let (Some(pattern), Some(callback)) = (steel_car(&item), steel_cdr(&item)) {
          if let SteelVal::StringV(s) = pattern {
            self.aliases.push(Alias { pattern: s.to_string(), callback });
          }
        }
      }
    }
    if let Ok(val) = self.engine.extract_value("*timers*") {
      for item in steel_list_iter(&val) {
        if let Some(fields) = steel_list_to_vec(&item) {
          if fields.len() >= 3 {
            let interval = steel_to_f64(&fields[0]).unwrap_or(1.0);
            let oneshot = matches!(&fields[1], SteelVal::BoolV(true));
            let callback = fields[2].clone();
            self.timers.push(Timer {
              interval_secs: interval,
              callback,
              last_fired_secs: now_secs(),
              oneshot
            });
          }
        }
      }
    }
  }

  pub fn handle_line(&mut self, line: &str) -> bool {
    let should_display = if let Some(on_line) = self.hooks.get("line").cloned() {
      match self.engine.call_function_with_args(on_line, vec![SteelVal::StringV(
        line.to_string().into()
      )]) {
        Ok(SteelVal::BoolV(false)) => false,
        Err(e) => {
          self.append_system_message(&format!("[line hook error: {e}]"));
          true
        }
        _ => true
      }
    } else {
      true
    };

    if should_display {
      for i in 0..self.triggers.len() {
        if let Ok(re) = regex::Regex::new(&self.triggers[i].pattern)
          && let Some(captures) = re.captures(line)
        {
          let args: Vec<SteelVal> = captures
            .iter()
            .skip(1)
            .filter_map(|m| m.map(|m| SteelVal::StringV(m.as_str().to_string().into())))
            .collect();
          let callback = self.triggers[i].callback.clone();
          if let Err(e) = self.engine.call_function_with_args(callback, args) {
            self.append_system_message(&format!("[trigger error: {e}]"));
          }
        }
      }
    }

    should_display
  }

  pub fn eval_input(&mut self, code: &str) {
    match self.engine.run(code.to_string()) {
      Ok(results) => {
        if let Some(val) = results.last() {
          let display = format!("{val}");
          if display != "Void" && !display.is_empty() {
            self.append_system_message(&format!("=> {display}"));
          }
        }
      }
      Err(e) => self.append_system_message(&format!("[eval error: {e}]"))
    }
    self.sync_registrations();
  }

  pub fn handle_input(&mut self, input: &str) -> bool {
    let matched = (0..self.aliases.len()).find_map(|i| {
      regex::Regex::new(&self.aliases[i].pattern)
        .ok()
        .and_then(|re| re.captures(input))
        .map(|captures| {
          let args: Vec<SteelVal> = captures
            .iter()
            .skip(1)
            .filter_map(|m| m.map(|m| SteelVal::StringV(m.as_str().to_string().into())))
            .collect();
          (self.aliases[i].callback.clone(), args)
        })
    });
    if let Some((callback, args)) = matched {
      if let Err(e) = self.engine.call_function_with_args(callback, args) {
        self.append_system_message(&format!("[alias error: {e}]"));
      }
      false
    } else {
      true
    }
  }

  fn call_hook(&mut self, name: &str, args: Vec<SteelVal>) {
    if let Some(func) = self.hooks.get(name).cloned()
      && let Err(e) = self.engine.call_function_with_args(func, args)
    {
      self.append_system_message(&format!("[{name} hook error: {e}]"));
    }
  }

  pub fn handle_gmcp(&mut self, package: &str, data: &serde_json::Value) {
    let steel_data = json_to_steel(data);
    self.call_hook("gmcp", vec![
      SteelVal::StringV(package.to_string().into()),
      steel_data,
    ]);
  }

  pub fn handle_msdp(&mut self, data: &serde_json::Value) {
    self.call_hook("msdp", vec![json_to_steel(data)]);
  }

  pub fn handle_input_hook(&mut self, input: &str) {
    self.call_hook("input", vec![SteelVal::StringV(input.to_string().into())]);
  }

  pub fn handle_connect(&mut self) {
    self.call_hook("connect", vec![]);
    self.sync_registrations();
  }

  pub fn handle_disconnect(&mut self) {
    self.call_hook("disconnect", vec![]);
    self.sync_registrations();
  }

  pub fn tick_timers(&mut self) {
    let now = now_secs();
    let mut to_remove = Vec::new();
    let mut errors = Vec::new();
    for (i, timer) in self.timers.iter_mut().enumerate() {
      if now - timer.last_fired_secs >= timer.interval_secs {
        timer.last_fired_secs = now;
        let callback = timer.callback.clone();
        if let Err(e) = self.engine.call_function_with_args(callback, vec![]) {
          errors.push(format!("[timer error: {e}]"));
        }
        if timer.oneshot {
          to_remove.push(i);
        }
      }
    }
    for i in to_remove.into_iter().rev() {
      self.timers.remove(i);
    }
    for msg in errors {
      self.append_system_message(&msg);
    }
  }

  pub fn trigger_count(&self) -> usize { self.triggers.len() }
  pub fn alias_count(&self) -> usize { self.aliases.len() }
  pub fn timer_count(&self) -> usize { self.timers.len() }

  pub fn drain_commands(&self) -> Vec<String> {
    std::mem::take(&mut self.state.lock().unwrap().outgoing_commands)
  }

  pub fn drain_gmcp(&self) -> Vec<(String, serde_json::Value)> {
    std::mem::take(&mut self.state.lock().unwrap().outgoing_gmcp)
  }

  pub fn drain_msdp_reports(&self) -> Vec<Vec<String>> {
    std::mem::take(&mut self.state.lock().unwrap().outgoing_msdp_report)
  }

  pub fn drain_msdp_sends(&self) -> Vec<Vec<String>> {
    std::mem::take(&mut self.state.lock().unwrap().outgoing_msdp_send)
  }

  pub fn drain_msdp_lists(&self) -> Vec<String> {
    std::mem::take(&mut self.state.lock().unwrap().outgoing_msdp_list)
  }

  pub fn append_to_main(&self, line: &str) {
    let mut st = self.state.lock().unwrap();
    let palette = st.ansi_palette;
    let styled_lines = parse_ansi(line, palette.as_ref());
    let main_buf =
      st.panes.entry("main".into()).or_insert_with(|| TextBuffer::new(10000));
    main_buf.append_lines(styled_lines);
  }

  pub fn set_main_pending(&self, line: &str) {
    let mut st = self.state.lock().unwrap();
    let palette = st.ansi_palette;
    let pending_line = parse_ansi(line, palette.as_ref()).into_iter().next();
    let main_buf =
      st.panes.entry("main".into()).or_insert_with(|| TextBuffer::new(10000));
    main_buf.pending_line = pending_line.filter(|line| !line.is_empty());
  }

  pub fn append_system_message(&self, msg: &str) {
    warn!("{msg}");
    let line = StyledLine::plain(msg);
    let mut st = self.state.lock().unwrap();
    let main_buf =
      st.panes.entry("main".into()).or_insert_with(|| TextBuffer::new(10000));
    main_buf.append_line(line);
  }
}

fn steel_list_iter(val: &SteelVal) -> Vec<SteelVal> {
  match val {
    SteelVal::ListV(list) => list.iter().cloned().collect(),
    _ => Vec::new()
  }
}

fn steel_car(val: &SteelVal) -> Option<SteelVal> {
  match val {
    SteelVal::Pair(p) => Some(p.car().clone()),
    SteelVal::ListV(list) => list.first().cloned(),
    _ => None
  }
}

fn steel_cdr(val: &SteelVal) -> Option<SteelVal> {
  match val {
    SteelVal::Pair(p) => Some(p.cdr().clone()),
    SteelVal::ListV(list) => list.get(1).cloned(),
    _ => None
  }
}

fn steel_list_to_vec(val: &SteelVal) -> Option<Vec<SteelVal>> {
  match val {
    SteelVal::ListV(list) => Some(list.iter().cloned().collect()),
    _ => None
  }
}

fn steel_to_f64(val: &SteelVal) -> Option<f64> {
  match val {
    SteelVal::NumV(n) => Some(*n),
    SteelVal::IntV(n) => Some(*n as f64),
    _ => None
  }
}

pub fn json_to_steel(val: &serde_json::Value) -> SteelVal {
  match val {
    serde_json::Value::Null => SteelVal::Void,
    serde_json::Value::Bool(b) => SteelVal::BoolV(*b),
    serde_json::Value::Number(n) => n
      .as_f64()
      .map(|f| {
        if f.fract() == 0.0 && f.abs() < isize::MAX as f64 {
          SteelVal::IntV(f as isize)
        } else {
          SteelVal::NumV(f)
        }
      })
      .unwrap_or(SteelVal::Void),
    serde_json::Value::String(s) => SteelVal::StringV(s.clone().into()),
    serde_json::Value::Array(arr) => {
      SteelVal::ListV(arr.iter().map(json_to_steel).collect())
    }
    serde_json::Value::Object(map) => {
      let mut hm = HashMap::new();
      for (k, v) in map {
        hm = hm.update(SteelVal::StringV(k.clone().into()), json_to_steel(v));
      }
      SteelVal::HashMapV(Gc::new(hm).into())
    }
  }
}

fn now_secs() -> f64 {
  #[cfg(target_arch = "wasm32")]
  {
    js_sys::Date::now() / 1000.0
  }
  #[cfg(not(target_arch = "wasm32"))]
  {
    static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    START.get_or_init(std::time::Instant::now).elapsed().as_secs_f64()
  }
}

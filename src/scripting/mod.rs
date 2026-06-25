pub mod api;

use std::sync::{Arc, Mutex};

use log::{debug, info, warn};
use mlua::prelude::*;
use eframe::egui::Color32;

use crate::buffer::{StyledLine, TextBuffer};
use crate::ansi::parse_ansi;

#[derive(Clone, Debug)]
pub struct Gauge {
    pub name: String,
    pub current: f64,
    pub max: f64,
    pub color: String,
}

pub struct Trigger {
    pub pattern: String,
    pub callback_key: LuaRegistryKey,
}

pub struct Alias {
    pub pattern: String,
    pub callback_key: LuaRegistryKey,
}

pub struct Timer {
    pub interval_secs: f64,
    pub callback_key: LuaRegistryKey,
    pub last_fired: std::time::Instant,
    pub oneshot: bool,
}

#[derive(Clone, Debug)]
pub struct LayoutEntry {
    pub pane: String,
    pub weight: f32,
}

#[derive(Clone, Debug)]
pub enum LayoutDir {
    Horizontal,
    Vertical,
}

#[derive(Clone, Debug)]
pub struct Layout {
    pub direction: LayoutDir,
    pub entries: Vec<LayoutEntry>,
}

#[derive(Clone, Debug)]
pub struct KeyCombo {
    pub alt: bool,
    pub ctrl: bool,
    pub shift: bool,
    pub key: String,
}

#[derive(Clone, Debug)]
pub struct Keymap {
    pub combo: KeyCombo,
    pub command: String,
}

pub struct ScriptState {
    pub panes: std::collections::HashMap<String, TextBuffer>,
    pub gauges: Vec<Gauge>,
    pub layout: Layout,
    pub outgoing_commands: Vec<String>,
    pub outgoing_gmcp: Vec<(String, serde_json::Value)>,
    pub outgoing_msdp_report: Vec<Vec<String>>,
    pub outgoing_msdp_send: Vec<Vec<String>>,
    pub keymaps: Vec<Keymap>,
    pub keep_input: bool,
    pub font_name: Option<String>,
    pub font_size: f32,
    pub bg_color: Option<[u8; 3]>,
    pub fg_color: Option<[u8; 3]>,
    pub ansi_palette: Option<[Color32; 16]>,
    pub profile_dir: Option<std::path::PathBuf>,
    pub status_text: String,
    pub theme_dirty: bool,
    pub scroll_lines: f32,
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
                entries: vec![LayoutEntry { pane: "main".into(), weight: 1.0 }],
            },
            outgoing_commands: Vec::new(),
            outgoing_gmcp: Vec::new(),
            outgoing_msdp_report: Vec::new(),
            outgoing_msdp_send: Vec::new(),
            keymaps: Vec::new(),
            keep_input: false,
            font_name: None,
            font_size: 13.0,
            bg_color: None,
            fg_color: None,
            ansi_palette: None,
            profile_dir: None,
            status_text: String::new(),
            theme_dirty: false,
            scroll_lines: 3.0,
        }
    }
}

pub struct ScriptEngine {
    lua: Lua,
    pub state: Arc<Mutex<ScriptState>>,
    triggers: Vec<Trigger>,
    aliases: Vec<Alias>,
    timers: Vec<Timer>,
    synced_trigger_count: usize,
    synced_alias_count: usize,
    synced_timer_count: usize,
}

impl ScriptEngine {
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new();
        let state = Arc::new(Mutex::new(ScriptState::new()));

        api::register_api(&lua, state.clone())?;

        Ok(Self {
            lua,
            state,
            triggers: Vec::new(),
            aliases: Vec::new(),
            timers: Vec::new(),
            synced_trigger_count: 0,
            synced_alias_count: 0,
            synced_timer_count: 0,
        })
    }

    pub fn load_script(&mut self, code: &str) -> LuaResult<()> {
        self.triggers.clear();
        self.aliases.clear();
        self.timers.clear();
        self.synced_trigger_count = 0;
        self.synced_alias_count = 0;
        self.synced_timer_count = 0;

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
        api::register_api(&self.lua, self.state.clone())?;

        self.lua.load(code).exec()?;
        self.sync_registrations();
        info!("Script loaded: {} triggers, {} aliases, {} timers",
            self.triggers.len(), self.aliases.len(), self.timers.len());
        let st = self.state.lock().unwrap();
        info!("Panes: {:?}, Gauges: {}, Layout entries: {}",
            st.panes.keys().collect::<Vec<_>>(), st.gauges.len(), st.layout.entries.len());

        Ok(())
    }

    fn sync_registrations(&mut self) {
        let globals = self.lua.globals();

        if let Ok(triggers) = globals.get::<LuaTable>("_triggers") {
            let count = triggers.len().unwrap_or(0) as usize;
            if count > self.synced_trigger_count {
                debug!("Syncing {} new triggers", count - self.synced_trigger_count);
            }
            for i in (self.synced_trigger_count + 1)..=count {
                if let Ok(v) = triggers.get::<LuaTable>(i as i64) {
                    if let (Ok(pattern), Ok(callback)) = (v.get::<String>("pattern"), v.get::<LuaFunction>("callback")) {
                        if let Ok(key) = self.lua.create_registry_value(callback) {
                            self.triggers.push(Trigger { pattern, callback_key: key });
                        }
                    }
                }
            }
            self.synced_trigger_count = count;
        }

        if let Ok(aliases) = globals.get::<LuaTable>("_aliases") {
            let count = aliases.len().unwrap_or(0) as usize;
            if count > self.synced_alias_count {
                debug!("Syncing {} new aliases", count - self.synced_alias_count);
            }
            for i in (self.synced_alias_count + 1)..=count {
                if let Ok(v) = aliases.get::<LuaTable>(i as i64) {
                    if let (Ok(pattern), Ok(callback)) = (v.get::<String>("pattern"), v.get::<LuaFunction>("callback")) {
                        if let Ok(key) = self.lua.create_registry_value(callback) {
                            self.aliases.push(Alias { pattern, callback_key: key });
                        }
                    }
                }
            }
            self.synced_alias_count = count;
        }

        if let Ok(timers) = globals.get::<LuaTable>("_timers") {
            let count = timers.len().unwrap_or(0) as usize;
            if count > self.synced_timer_count {
                debug!("Syncing {} new timers", count - self.synced_timer_count);
            }
            for i in (self.synced_timer_count + 1)..=count {
                if let Ok(v) = timers.get::<LuaTable>(i as i64) {
                    let interval: f64 = v.get("interval").unwrap_or(1.0);
                    let oneshot: bool = v.get("oneshot").unwrap_or(false);
                    if let Ok(callback) = v.get::<LuaFunction>("callback") {
                        if let Ok(key) = self.lua.create_registry_value(callback) {
                            self.timers.push(Timer {
                                interval_secs: interval,
                                callback_key: key,
                                last_fired: std::time::Instant::now(),
                                oneshot,
                            });
                        }
                    }
                }
            }
            self.synced_timer_count = count;
        }
    }

    pub fn handle_line(&mut self, line: &str) -> bool {
        let globals = self.lua.globals();

        if let Ok(on_line) = globals.get::<LuaFunction>("on_line") {
            match on_line.call::<LuaValue>(line.to_string()) {
                Ok(LuaValue::Boolean(false)) => return false,
                Ok(_) => {}
                Err(e) => self.append_system_message(&format!("[on_line error: {e}]")),
            }
        }

        for trigger in &self.triggers {
            if let Ok(re) = regex::Regex::new(&trigger.pattern) {
                if let Some(captures) = re.captures(line) {
                    let callback: LuaFunction = self.lua.registry_value(&trigger.callback_key).unwrap();
                    let args: Vec<String> = captures
                        .iter()
                        .skip(1)
                        .filter_map(|m| m.map(|m| m.as_str().to_string()))
                        .collect();
                    let lua_args = self.lua.create_sequence_from(args).unwrap();
                    if let Err(e) = callback.call::<()>(LuaMultiValue::from_vec(
                        lua_args.sequence_values::<LuaValue>().filter_map(|v| v.ok()).collect(),
                    )) {
                        self.append_system_message(&format!("[trigger error: {e}]"));
                    }
                }
            }
        }

        true
    }

    pub fn handle_input(&mut self, input: &str) -> bool {
        for alias in &self.aliases {
            if let Ok(re) = regex::Regex::new(&alias.pattern) {
                if let Some(captures) = re.captures(input) {
                    let callback: LuaFunction = self.lua.registry_value(&alias.callback_key).unwrap();
                    let args: Vec<String> = captures
                        .iter()
                        .skip(1)
                        .filter_map(|m| m.map(|m| m.as_str().to_string()))
                        .collect();
                    let lua_args = self.lua.create_sequence_from(args).unwrap();
                    if let Err(e) = callback.call::<()>(LuaMultiValue::from_vec(
                        lua_args.sequence_values::<LuaValue>().filter_map(|v| v.ok()).collect(),
                    )) {
                        self.append_system_message(&format!("[alias error: {e}]"));
                    }
                    return false;
                }
            }
        }
        true
    }

    pub fn handle_gmcp(&mut self, package: &str, data: &serde_json::Value) {
        let globals = self.lua.globals();
        if let Ok(on_gmcp) = globals.get::<LuaFunction>("on_gmcp") {
            let lua_data = self.lua.to_value(data).unwrap_or(LuaValue::Nil);
            if let Err(e) = on_gmcp.call::<()>((package.to_string(), lua_data)) {
                self.append_system_message(&format!("[on_gmcp error: {e}]"));
            }
        }
    }

    pub fn handle_msdp(&mut self, data: &serde_json::Value) {
        let globals = self.lua.globals();
        if let Ok(on_msdp) = globals.get::<LuaFunction>("on_msdp") {
            let lua_data = self.lua.to_value(data).unwrap_or(LuaValue::Nil);
            if let Err(e) = on_msdp.call::<()>(lua_data) {
                self.append_system_message(&format!("[on_msdp error: {e}]"));
            }
        }
    }

    pub fn handle_input_hook(&mut self, input: &str) {
        let globals = self.lua.globals();
        if let Ok(on_input) = globals.get::<LuaFunction>("on_input") {
            if let Err(e) = on_input.call::<()>(input.to_string()) {
                self.append_system_message(&format!("[on_input error: {e}]"));
            }
        }
    }

    pub fn handle_connect(&mut self) {
        let globals = self.lua.globals();
        if let Ok(f) = globals.get::<LuaFunction>("on_connect") {
            if let Err(e) = f.call::<()>(()) {
                self.append_system_message(&format!("[on_connect error: {e}]"));
            }
        }
        self.sync_registrations();
    }

    pub fn handle_disconnect(&mut self) {
        let globals = self.lua.globals();
        if let Ok(f) = globals.get::<LuaFunction>("on_disconnect") {
            if let Err(e) = f.call::<()>(()) {
                self.append_system_message(&format!("[on_disconnect error: {e}]"));
            }
        }
        self.sync_registrations();
    }

    pub fn tick_timers(&mut self) {
        self.sync_registrations();
        let now = std::time::Instant::now();
        let mut to_remove = Vec::new();
        let mut errors = Vec::new();
        for (i, timer) in self.timers.iter_mut().enumerate() {
            if now.duration_since(timer.last_fired).as_secs_f64() >= timer.interval_secs {
                timer.last_fired = now;
                let callback: LuaFunction = self.lua.registry_value(&timer.callback_key).unwrap();
                if let Err(e) = callback.call::<()>(()) {
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
        self.sync_registrations();
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

    pub fn append_to_main(&self, line: &str) {
        let mut st = self.state.lock().unwrap();
        let palette = st.ansi_palette;
        let styled_lines = parse_ansi(line, palette.as_ref());
        let main_buf = st.panes.entry("main".into()).or_insert_with(|| TextBuffer::new(10000));
        main_buf.append_lines(styled_lines);
    }

    pub fn append_system_message(&self, msg: &str) {
        warn!("{msg}");
        let line = StyledLine::plain(msg);
        let mut st = self.state.lock().unwrap();
        let main_buf = st.panes.entry("main".into()).or_insert_with(|| TextBuffer::new(10000));
        main_buf.append_line(line);
    }
}

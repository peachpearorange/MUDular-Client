use std::sync::{Arc, Mutex};

use mlua::prelude::*;

use eframe::egui::Color32;

use crate::ansi::{parse_ansi, strip_ansi, DEFAULT_PALETTE};
use crate::buffer::TextBuffer;
use crate::scripting::{LayoutDir, LayoutEntry, Layout, ScriptState};

pub fn register_api(lua: &Lua, state: Arc<Mutex<ScriptState>>) -> LuaResult<()> {
    let mud = lua.create_table()?;

    // mud.pane(name) -> PaneHandle
    let pane_state = state.clone();
    mud.set(
        "pane",
        lua.create_function(move |lua, name: String| {
            {
                let mut st = pane_state.lock().unwrap();
                st.panes.entry(name.clone()).or_insert_with(|| TextBuffer::new(10000));
            }
            let handle = lua.create_table()?;
            handle.set("_name", name)?;
            Ok(handle)
        })?,
    )?;

    // mud.send(text)
    let send_state = state.clone();
    mud.set(
        "send",
        lua.create_function(move |_, text: String| {
            send_state.lock().unwrap().outgoing_commands.push(text);
            Ok(())
        })?,
    )?;

    // mud.send_gmcp(package, data)
    let gmcp_state = state.clone();
    mud.set(
        "send_gmcp",
        lua.create_function(move |_, (package, data): (String, LuaValue)| {
            let json = lua_to_json(&data);
            gmcp_state.lock().unwrap().outgoing_gmcp.push((package, json));
            Ok(())
        })?,
    )?;

    // mud.gauge(name, {current, max, color})
    let gauge_state = state.clone();
    mud.set(
        "gauge",
        lua.create_function(move |_, (name, opts): (String, LuaTable)| {
            let current: Option<f64> = opts.get("current").ok();
            let max: Option<f64> = opts.get("max").ok();
            let color: Option<String> = opts.get("color").ok();

            let mut st = gauge_state.lock().unwrap();
            if let Some(g) = st.gauges.iter_mut().find(|g| g.name == name) {
                if let Some(v) = current { g.current = v; }
                if let Some(v) = max { g.max = v; }
                if let Some(v) = color { g.color = v; }
            } else {
                st.gauges.push(crate::scripting::Gauge {
                    name,
                    current: current.unwrap_or(0.0),
                    max: max.unwrap_or(100.0),
                    color: color.unwrap_or_else(|| "green".into()),
                });
            }
            Ok(())
        })?,
    )?;

    // mud.layout(direction, entries)
    let layout_state = state.clone();
    mud.set(
        "layout",
        lua.create_function(move |_, (dir, entries): (String, LuaTable)| {
            let direction = match dir.as_str() {
                "vertical" => LayoutDir::Vertical,
                _ => LayoutDir::Horizontal,
            };
            let mut layout_entries = Vec::new();
            entries.for_each(|_k: LuaValue, v: LuaTable| {
                let pane: String = v.get("pane").unwrap_or_default();
                let weight: f32 = v.get("weight").unwrap_or(1.0);
                layout_entries.push(LayoutEntry { pane, weight });
                Ok(())
            })?;
            layout_state.lock().unwrap().layout = Layout { direction, entries: layout_entries };
            Ok(())
        })?,
    )?;

    // mud.strip_ansi(text)
    mud.set(
        "strip_ansi",
        lua.create_function(|_, text: String| Ok(strip_ansi(&text)))?,
    )?;

    // mud.keymap(combo_str, command) — bind a key combo to a command
    let keymap_state = state.clone();
    mud.set(
        "keymap",
        lua.create_function(move |_, (combo_str, command): (String, String)| {
            let combo = parse_key_combo(&combo_str);
            keymap_state.lock().unwrap().keymaps.push(
                crate::scripting::Keymap { combo, command },
            );
            Ok(())
        })?,
    )?;

    // mud.option(name, value) — set a client option
    let opt_state = state.clone();
    mud.set(
        "option",
        lua.create_function(move |_, (name, value): (String, LuaValue)| {
            let mut st = opt_state.lock().unwrap();
            match name.as_str() {
                "keep_input" => st.keep_input = matches!(value, LuaValue::Boolean(true)),
                "font" => {
                    if let LuaValue::String(s) = value {
                        st.font_name = Some(s.to_string_lossy().to_string());
                        st.theme_dirty = true;
                    }
                }
                "font_size" => {
                    let size = match value {
                        LuaValue::Number(n) => n as f32,
                        LuaValue::Integer(n) => n as f32,
                        _ => st.font_size,
                    };
                    st.font_size = size;
                    st.theme_dirty = true;
                }
                "bg_color" => {
                    st.bg_color = parse_lua_color(&value);
                    st.theme_dirty = true;
                }
                "fg_color" => {
                    st.fg_color = parse_lua_color(&value);
                    st.theme_dirty = true;
                }
                "scroll_lines" => {
                    st.scroll_lines = match value {
                        LuaValue::Number(n) => n as f32,
                        LuaValue::Integer(n) => n as f32,
                        _ => st.scroll_lines,
                    };
                }
                _ => {}
            }
            Ok(())
        })?,
    )?;

    // mud.colors({bg = "#...", fg = "#..."})
    let colors_state = state.clone();
    mud.set(
        "colors",
        lua.create_function(move |_, opts: LuaTable| {
            let mut st = colors_state.lock().unwrap();
            if let Ok(bg) = opts.get::<LuaValue>("bg") {
                st.bg_color = parse_lua_color(&bg);
            }
            if let Ok(fg) = opts.get::<LuaValue>("fg") {
                st.fg_color = parse_lua_color(&fg);
            }
            st.theme_dirty = true;
            Ok(())
        })?,
    )?;

    // mud.load_theme(name_or_path) — load a built-in theme by name or a Kitty .conf file by path
    let theme_state = state.clone();
    mud.set(
        "load_theme",
        lua.create_function(move |_, name_or_path: String| {
            let mut st = theme_state.lock().unwrap();
            if let Some(content) = crate::themes::get_builtin_theme(&name_or_path) {
                parse_kitty_theme(content, &mut st);
                st.theme_dirty = true;
                Ok(())
            } else {
                let resolved = if std::path::Path::new(&name_or_path).is_absolute() {
                    std::path::PathBuf::from(&name_or_path)
                } else {
                    st.profile_dir.as_ref()
                        .map(|d| d.join(&name_or_path))
                        .unwrap_or_else(|| std::path::PathBuf::from(&name_or_path))
                };
                match std::fs::read_to_string(&resolved) {
                    Ok(content) => {
                        parse_kitty_theme(&content, &mut st);
                        st.theme_dirty = true;
                        Ok(())
                    }
                    Err(e) => Err(LuaError::external(format!(
                        "Unknown built-in theme and failed to read file '{}': {e}",
                        resolved.display()
                    ))),
                }
            }
        })?,
    )?;

    let status_state = state.clone();
    mud.set(
        "status",
        lua.create_function(move |_, text: String| {
            status_state.lock().unwrap().status_text = text;
            Ok(())
        })?,
    )?;

    // mud.msdp_report(vars) — request MSDP REPORT for a list of variable names
    let msdp_state = state.clone();
    mud.set(
        "msdp_report",
        lua.create_function(move |_, vars: LuaTable| {
            let mut var_list = Vec::new();
            vars.for_each(|_k: LuaValue, v: LuaValue| {
                if let LuaValue::String(s) = v {
                    var_list.push(s.to_string_lossy().to_string());
                }
                Ok(())
            })?;
            msdp_state.lock().unwrap().outgoing_msdp_report.push(var_list);
            Ok(())
        })?,
    )?;

    // mud.msdp_send(vars) — one-time request for current values of MSDP variables
    let msdp_send_state = state.clone();
    mud.set(
        "msdp_send",
        lua.create_function(move |_, vars: LuaTable| {
            let mut var_list = Vec::new();
            vars.for_each(|_k: LuaValue, v: LuaValue| {
                if let LuaValue::String(s) = v {
                    var_list.push(s.to_string_lossy().to_string());
                }
                Ok(())
            })?;
            msdp_send_state.lock().unwrap().outgoing_msdp_send.push(var_list);
            Ok(())
        })?,
    )?;

    // mud.trigger(pattern, callback)
    mud.set(
        "trigger",
        lua.create_function(|lua, (pattern, callback): (String, LuaFunction)| {
            let globals = lua.globals();
            let triggers: LuaTable = globals
                .get("_triggers")
                .unwrap_or_else(|_| lua.create_table().unwrap());
            let entry = lua.create_table()?;
            entry.set("pattern", pattern)?;
            entry.set("callback", callback)?;
            let len = triggers.len()? + 1;
            triggers.set(len, entry)?;
            globals.set("_triggers", triggers)?;
            Ok(())
        })?,
    )?;

    // mud.alias(pattern, callback)
    mud.set(
        "alias",
        lua.create_function(|lua, (pattern, callback): (String, LuaFunction)| {
            let globals = lua.globals();
            let aliases: LuaTable = globals
                .get("_aliases")
                .unwrap_or_else(|_| lua.create_table().unwrap());
            let entry = lua.create_table()?;
            entry.set("pattern", pattern)?;
            entry.set("callback", callback)?;
            let len = aliases.len()? + 1;
            aliases.set(len, entry)?;
            globals.set("_aliases", aliases)?;
            Ok(())
        })?,
    )?;

    // mud.timer(interval, callback) — oneshot timer (fires once)
    mud.set(
        "timer",
        lua.create_function(|lua, (interval, callback): (f64, LuaFunction)| {
            let globals = lua.globals();
            let timers: LuaTable = globals
                .get("_timers")
                .unwrap_or_else(|_| lua.create_table().unwrap());
            let entry = lua.create_table()?;
            entry.set("interval", interval)?;
            entry.set("callback", callback)?;
            entry.set("oneshot", true)?;
            let len = timers.len()? + 1;
            timers.set(len, entry)?;
            globals.set("_timers", timers)?;
            Ok(())
        })?,
    )?;

    // mud.interval(interval, callback) — repeating timer
    mud.set(
        "interval",
        lua.create_function(|lua, (interval, callback): (f64, LuaFunction)| {
            let globals = lua.globals();
            let timers: LuaTable = globals
                .get("_timers")
                .unwrap_or_else(|_| lua.create_table().unwrap());
            let entry = lua.create_table()?;
            entry.set("interval", interval)?;
            entry.set("callback", callback)?;
            entry.set("oneshot", false)?;
            let len = timers.len()? + 1;
            timers.set(len, entry)?;
            globals.set("_timers", timers)?;
            Ok(())
        })?,
    )?;

    // Pane methods registered as global functions that take a pane table
    // pane:print(text), pane:cprint(text), pane:clear()
    let print_state = state.clone();
    lua.globals().set(
        "_pane_print",
        lua.create_function(move |_, (pane_table, text): (LuaTable, String)| {
            let name: String = pane_table.get("_name")?;
            let mut st = print_state.lock().unwrap();
            let palette = st.ansi_palette;
            let lines = parse_ansi(&text, palette.as_ref());
            let buf = st.panes.entry(name).or_insert_with(|| TextBuffer::new(10000));
            buf.append_lines(lines);
            Ok(())
        })?,
    )?;

    let clear_state = state.clone();
    lua.globals().set(
        "_pane_clear",
        lua.create_function(move |_, pane_table: LuaTable| {
            let name: String = pane_table.get("_name")?;
            let mut st = clear_state.lock().unwrap();
            if let Some(buf) = st.panes.get_mut(&name) {
                buf.clear();
            }
            Ok(())
        })?,
    )?;

    lua.globals().set("mud", mud)?;

    // Set up metatable for pane handles so pane:print() works
    lua.load(
        r#"
        _pane_mt = {
            __index = {
                print = function(self, text) _pane_print(self, text) end,
                cprint = function(self, text) _pane_print(self, text) end,
                clear = function(self) _pane_clear(self) end,
            }
        }
        local orig_pane = mud.pane
        mud.pane = function(name)
            local p = orig_pane(name)
            setmetatable(p, _pane_mt)
            return p
        end
        _triggers = {}
        _aliases = {}
        _timers = {}
        "#,
    )
    .exec()?;

    Ok(())
}

fn parse_lua_color(value: &LuaValue) -> Option<[u8; 3]> {
    match value {
        LuaValue::String(s) => parse_hex_color(&s.to_string_lossy()),
        LuaValue::Table(t) => {
            let r: u8 = t.get(1).unwrap_or(0);
            let g: u8 = t.get(2).unwrap_or(0);
            let b: u8 = t.get(3).unwrap_or(0);
            Some([r, g, b])
        }
        _ => None,
    }
}

fn parse_hex_color(s: &str) -> Option<[u8; 3]> {
    let s = s.strip_prefix('#').unwrap_or(s);
    if s.len() != 6 {
        None
    } else {
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some([r, g, b])
    }
}

fn parse_key_combo(s: &str) -> crate::scripting::KeyCombo {
    let mut alt = false;
    let mut ctrl = false;
    let mut shift = false;
    let mut key = String::new();
    for part in s.split('+') {
        let p = part.trim().to_lowercase();
        match p.as_str() {
            "alt" => alt = true,
            "ctrl" => ctrl = true,
            "shift" => shift = true,
            _ => key = p,
        }
    }
    crate::scripting::KeyCombo { alt, ctrl, shift, key }
}

fn parse_kitty_theme(content: &str, state: &mut ScriptState) {
    let mut palette = state.ansi_palette.unwrap_or(DEFAULT_PALETTE);
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.splitn(2, char::is_whitespace);
        let Some(key) = parts.next() else { continue };
        let Some(value) = parts.next().map(|s| s.trim()) else { continue };
        match key {
            "background" => { state.bg_color = parse_hex_color(value); }
            "foreground" => { state.fg_color = parse_hex_color(value); }
            k if k.starts_with("color") => {
                if let Some(idx) = k.strip_prefix("color").and_then(|s| s.parse::<usize>().ok()) {
                    if idx < 16 {
                        if let Some([r, g, b]) = parse_hex_color(value) {
                            palette[idx] = Color32::from_rgb(r, g, b);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    state.ansi_palette = Some(palette);
}

fn lua_to_json(val: &LuaValue) -> serde_json::Value {
    match val {
        LuaValue::Nil => serde_json::Value::Null,
        LuaValue::Boolean(b) => serde_json::Value::Bool(*b),
        LuaValue::Integer(n) => serde_json::json!(n),
        LuaValue::Number(n) => serde_json::json!(n),
        LuaValue::String(s) => serde_json::Value::String(s.to_string_lossy().to_string()),
        LuaValue::Table(t) => {
            let mut is_array = true;
            let mut max_key = 0i64;
            t.for_each(|k: LuaValue, _v: LuaValue| {
                match k {
                    LuaValue::Integer(i) if i > 0 => max_key = max_key.max(i),
                    _ => is_array = false,
                }
                Ok(())
            })
            .unwrap_or(());

            if is_array && max_key > 0 {
                let arr: Vec<serde_json::Value> = (1..=max_key)
                    .filter_map(|i| t.get::<LuaValue>(i).ok().map(|v| lua_to_json(&v)))
                    .collect();
                serde_json::Value::Array(arr)
            } else {
                let mut map = serde_json::Map::new();
                t.for_each(|k: LuaValue, v: LuaValue| {
                    let key = match &k {
                        LuaValue::String(s) => s.to_string_lossy().to_string(),
                        LuaValue::Integer(n) => n.to_string(),
                        _ => return Ok(()),
                    };
                    map.insert(key, lua_to_json(&v));
                    Ok(())
                })
                .unwrap_or(());
                serde_json::Value::Object(map)
            }
        }
        _ => serde_json::Value::Null,
    }
}

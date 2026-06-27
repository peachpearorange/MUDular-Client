use std::sync::{Arc, Mutex};

use steel::{rvals::SteelVal,
            steel_vm::{engine::Engine, register_fn::RegisterFn}};

use eframe::egui::Color32;

use crate::{ansi::{DEFAULT_PALETTE, parse_ansi, strip_ansi},
            buffer::{StyledLine, TextBuffer},
            scripting::{Gauge, Layout, LayoutDir, LayoutEntry, ScriptState}};

pub fn register_api(engine: &mut Engine, state: Arc<Mutex<ScriptState>>) {
  macro_rules! reg {
    ($name:expr, $state:ident => $body:expr) => {{
      let $state = state.clone();
      engine.register_fn($name, $body);
    }};
  }

  reg!("mud/pane", s => move |name: String| -> String {
      s.lock().unwrap().panes.entry(name.clone()).or_insert_with(|| TextBuffer::new(10000));
      name
  });

  reg!("mud/profile*", s => move |args: SteelVal| {
      let mut st = s.lock().unwrap();
      for pair in steel_list_to_vec(&args).chunks(2) {
          if let [key, value] = pair {
              match steel_key_name(key).as_deref() {
                  Some("name") => st.profile_name = steel_to_string(value),
                  Some("connection-mode") => st.profile_connection_mode = steel_key_name(value),
                  Some("host") => st.profile_host = steel_to_string(value),
                  Some("port") => st.profile_port = steel_to_f64(value).map(|n| n as u16),
                  Some("tls") => st.profile_tls = Some(matches!(value, SteelVal::BoolV(true))),
                  Some("websocket-url") => st.profile_websocket_url = steel_to_string(value),
                  Some("websocket-protocol") => {
                      st.profile_websocket_protocol = steel_to_string(value);
                  }
                  _ => {}
              }
          }
      }
  });

  reg!("mud/pane-print", s => move |name: String, text: String| {
      let mut st = s.lock().unwrap();
      let palette = st.ansi_palette;
      let text = restore_escaped_ansi(&text);
      let lines = parse_ansi(&text, palette.as_ref());
      let buf = st.panes.entry(name).or_insert_with(|| TextBuffer::new(10000));
      buf.append_lines(lines);
  });

  reg!("mud/pane-clear", s => move |name: String| {
      if let Some(buf) = s.lock().unwrap().panes.get_mut(&name) { buf.clear(); }
  });

  reg!("mud/send", s => move |text: String| {
      s.lock().unwrap().outgoing_commands.push(text);
  });

  reg!("mud/reconnect", s => move || {
      s.lock().unwrap().outgoing_reconnect = true;
  });

  engine.register_fn("mud/strip-ansi", |text: String| -> String { strip_ansi(&text) });

  engine.register_fn("mud/regexp-match?", |pattern: String, text: String| -> bool {
    regex::Regex::new(&pattern).map(|re| re.is_match(&text)).unwrap_or(false)
  });

  engine.register_fn("mud/fonts", || -> SteelVal {
    SteelVal::ListV(
      crate::fonts::available_fonts()
        .into_iter()
        .map(|font| SteelVal::StringV(font.into()))
        .collect()
    )
  });

  engine.register_fn("mud/themes", || -> SteelVal {
    SteelVal::ListV(
      crate::themes::theme_names().map(|theme| SteelVal::StringV(theme.into())).collect()
    )
  });

  reg!("mud/scroll-up", s => move |lines: SteelVal| {
      let lines = steel_to_f32(&lines).unwrap_or(10.0);
      let mut st = s.lock().unwrap();
      if let Some(buf) = st.panes.get_mut("main") {
          buf.scroll_delta_lines += lines;
          buf.auto_scroll = false;
      }
  });

  reg!("mud/scroll-down", s => move |lines: SteelVal| {
      let lines = steel_to_f32(&lines).unwrap_or(10.0);
      let mut st = s.lock().unwrap();
      if let Some(buf) = st.panes.get_mut("main") {
          buf.scroll_delta_lines -= lines;
      }
  });

  reg!("mud/capture-key", s => move || {
      let mut st = s.lock().unwrap();
      st.capture_next_key = true;
      let color = st.ansi_palette.unwrap_or(DEFAULT_PALETTE)[3];
      let line = StyledLine::foreground("[Key combo capture active — press a key combo...]", color);
      st.panes.entry("main".into()).or_insert_with(|| TextBuffer::new(10000)).append_line(line);
  });

  reg!("mud/status", s => move |text: String| {
      let mut st = s.lock().unwrap();
      let palette = st.ansi_palette;
      let text = restore_escaped_ansi(&text);
      st.status_line = parse_ansi(&text, palette.as_ref())
          .into_iter()
          .next()
          .unwrap_or_else(|| crate::buffer::StyledLine { spans: Vec::new() });
  });

  reg!("mud/gauge", s => move |name: String, opts: SteelVal| {
      let (current, max, color) = parse_gauge_opts(&opts);
      let mut st = s.lock().unwrap();
      if let Some(g) = st.gauges.iter_mut().find(|g| g.name == name) {
          if let Some(v) = current { g.current = v; }
          if let Some(v) = max { g.max = v; }
          if let Some(v) = color { g.color = v; }
      } else {
          st.gauges.push(Gauge {
              name,
              current: current.unwrap_or(0.0),
              max: max.unwrap_or(100.0),
              color: color.unwrap_or_else(|| "green".into()),
          });
      }
  });

  reg!("mud/layout", s => move |dir: String, entries: SteelVal| {
      let direction = match dir.as_str() {
          "vertical" => LayoutDir::Vertical,
          _ => LayoutDir::Horizontal,
      };
      let mut layout_entries = Vec::new();
      if let SteelVal::ListV(list) = &entries {
          for item in list.iter() {
              if let SteelVal::ListV(pair) = item {
                  let items: Vec<_> = pair.iter().collect();
                  if items.len() >= 2 {
                      if let SteelVal::StringV(pane) = &items[0] {
                          let weight = steel_to_f32(&items[1]).unwrap_or(1.0);
                          layout_entries.push(LayoutEntry { pane: pane.to_string(), weight });
                      }
                  }
              }
          }
      }
      s.lock().unwrap().layout = Layout { direction, entries: layout_entries };
  });

  reg!("mud/option", s => move |name: String, value: SteelVal| {
      let mut st = s.lock().unwrap();
      match name.as_str() {
          "keep_input" => st.keep_input = matches!(value, SteelVal::BoolV(true)),
          "font" => if let SteelVal::StringV(v) = &value {
              st.font_name = Some(v.to_string());
              st.theme_dirty = true;
          },
          "font_size" => {
              st.font_size = steel_to_f32(&value).unwrap_or(st.font_size);
              st.theme_dirty = true;
          }
          "bg_color" => { st.bg_color = parse_color(&value); st.theme_dirty = true; }
          "fg_color" => { st.fg_color = parse_color(&value); st.theme_dirty = true; }
          "scroll_lines" => {
              st.scroll_lines = steel_to_f32(&value).unwrap_or(st.scroll_lines);
          }
          _ => {}
      }
  });

  reg!("mud/load-theme", s => move |name: String| -> Result<(), String> {
      let mut st = s.lock().unwrap();
      if let Some(content) = crate::themes::get_builtin_theme(&name) {
          parse_kitty_theme(content, &mut st);
          st.theme_dirty = true;
          Ok(())
      } else {
          Err(format!(
              "Unknown theme '{}'. Use /(mud/themes) to see available built-in themes.", name
          ))
      }
  });

  reg!("mud/send-gmcp", s => move |package: String, data: SteelVal| {
      s.lock().unwrap().outgoing_gmcp.push((package, steel_to_json(&data)));
  });

  reg!("mud/msdp-report", s => move |vars: SteelVal| {
      s.lock().unwrap().outgoing_msdp_report.push(steel_to_string_list(&vars));
  });

  reg!("mud/msdp-send", s => move |vars: SteelVal| {
      s.lock().unwrap().outgoing_msdp_send.push(steel_to_string_list(&vars));
  });

  reg!("mud/msdp-list", s => move |what: String| {
      s.lock().unwrap().outgoing_msdp_list.push(what);
  });

  let mut prelude = PRELUDE.to_string();
  for name in crate::themes::theme_names() {
      let sym = crate::themes::theme_symbol(name);
      prelude.push_str(&format!("(define {} {:?})\n", sym, name));
  }
  engine.run(prelude).expect("failed to load scripting prelude");
}

const PRELUDE: &str = r#"
(define *triggers* '())
(define *aliases* '())
(define *timers* '())
(define *hooks* (hash))
(define *keymaps* (hash))

(define (trigger pattern callback)
  (set! *triggers* (cons (cons pattern callback) *triggers*)))

(define (alias pattern callback)
  (set! *aliases* (cons (cons pattern callback) *aliases*)))

(define (timer interval callback)
  (set! *timers* (cons (list interval #t callback) *timers*)))

(define (interval secs callback)
  (set! *timers* (cons (list secs #f callback) *timers*)))

(define (mud/profile . args)
  (mud/profile* args))

(define (mud/on event-name callback)
  (set! *hooks* (hash-insert *hooks* event-name callback)))

(define (mud/keymap combo callback)
  (set! *keymaps* (hash-insert *keymaps* combo callback)))

(define (hash-get h key . default)
  (if (hash-contains? h key)
      (hash-ref h key)
      (if (null? default) (void) (car default))))
"#;

fn parse_gauge_opts(opts: &SteelVal) -> (Option<f64>, Option<f64>, Option<String>) {
  match opts {
    SteelVal::HashMapV(hm) => {
      let get_f64 = |key: &str| hash_get_by_name(hm, key).and_then(steel_to_f64);
      let color = hash_get_by_name(hm, "color").and_then(steel_to_string);
      (get_f64("current"), get_f64("max"), color)
    }
    _ => (None, None, None)
  }
}

fn steel_list_to_vec(val: &SteelVal) -> Vec<SteelVal> {
  match val {
    SteelVal::ListV(list) => list.iter().cloned().collect(),
    _ => Vec::new()
  }
}

fn restore_escaped_ansi(text: &str) -> String {
  text.replace("\\u{1b}", "\x1b").replace("\\u{001b}", "\x1b").replace("\\x1b", "\x1b")
}

fn hash_get_by_name<'a>(
  hm: &'a steel::HashMap<SteelVal, SteelVal>,
  key: &str
) -> Option<&'a SteelVal> {
  hm.iter()
    .find(|(k, _)| steel_key_name(k).is_some_and(|name| name == key))
    .map(|(_, v)| v)
}

fn steel_key_name(val: &SteelVal) -> Option<String> {
  match val {
    SteelVal::StringV(s) | SteelVal::SymbolV(s) => {
      Some(s.to_string().trim_start_matches("##").to_string())
    }
    _ => None
  }
}

fn steel_to_string(val: &SteelVal) -> Option<String> {
  match val {
    SteelVal::StringV(s) | SteelVal::SymbolV(s) => Some(s.to_string()),
    _ => None
  }
}

fn steel_to_f64(val: &SteelVal) -> Option<f64> {
  match val {
    SteelVal::NumV(n) => Some(*n),
    SteelVal::IntV(n) => Some(*n as f64),
    SteelVal::StringV(s) => s.parse::<f64>().ok(),
    _ => None
  }
}

fn steel_to_f32(val: &SteelVal) -> Option<f32> { steel_to_f64(val).map(|n| n as f32) }

fn steel_to_string_list(val: &SteelVal) -> Vec<String> {
  match val {
    SteelVal::ListV(list) => list
      .iter()
      .filter_map(|v| match v {
        SteelVal::StringV(s) => Some(s.to_string()),
        _ => None
      })
      .collect(),
    _ => Vec::new()
  }
}

fn parse_color(value: &SteelVal) -> Option<[u8; 3]> {
  match value {
    SteelVal::StringV(s) => parse_hex_color(s.as_str()),
    SteelVal::ListV(list) => {
      let items: Vec<_> = list.iter().collect();
      if items.len() >= 3 {
        Some([
          steel_to_f64(&items[0])? as u8,
          steel_to_f64(&items[1])? as u8,
          steel_to_f64(&items[2])? as u8
        ])
      } else {
        None
      }
    }
    _ => None
  }
}

fn parse_hex_color(s: &str) -> Option<[u8; 3]> {
  let s = s.strip_prefix('#').unwrap_or(s);
  if s.len() != 6 {
    None
  } else {
    Some([
      u8::from_str_radix(&s[0..2], 16).ok()?,
      u8::from_str_radix(&s[2..4], 16).ok()?,
      u8::from_str_radix(&s[4..6], 16).ok()?
    ])
  }
}

fn parse_kitty_theme(content: &str, state: &mut ScriptState) {
  let mut palette = state.ansi_palette.unwrap_or(DEFAULT_PALETTE);
  for (key, value) in content
    .lines()
    .map(str::trim)
    .filter(|line| !line.is_empty() && !line.starts_with('#'))
    .filter_map(|line| {
      let mut parts = line.splitn(2, char::is_whitespace);
      parts.next().zip(parts.next().map(str::trim))
    })
  {
    match key {
      "background" => state.bg_color = parse_hex_color(value),
      "foreground" => state.fg_color = parse_hex_color(value),
      k if k.starts_with("color") => {
        if let Some(idx) = k.strip_prefix("color").and_then(|s| s.parse::<usize>().ok())
          && idx < 16
          && let Some([r, g, b]) = parse_hex_color(value)
        {
          palette[idx] = Color32::from_rgb(r, g, b);
        }
      }
      _ => {}
    }
  }
  state.ansi_palette = Some(palette);
}

fn steel_to_json(val: &SteelVal) -> serde_json::Value {
  match val {
    SteelVal::Void => serde_json::Value::Null,
    SteelVal::BoolV(b) => serde_json::Value::Bool(*b),
    SteelVal::IntV(n) => serde_json::json!(n),
    SteelVal::NumV(n) => serde_json::json!(n),
    SteelVal::StringV(s) => serde_json::Value::String(s.to_string()),
    SteelVal::ListV(list) => {
      serde_json::Value::Array(list.iter().map(steel_to_json).collect())
    }
    SteelVal::HashMapV(hm) => serde_json::Value::Object(
      hm.iter()
        .filter_map(|(k, v)| {
          match k {
            SteelVal::StringV(s) => Some(s.to_string()),
            SteelVal::SymbolV(s) => Some(s.to_string()),
            SteelVal::IntV(n) => Some(n.to_string()),
            _ => None
          }
          .map(|key| (key, steel_to_json(v)))
        })
        .collect()
    ),
    _ => serde_json::Value::Null
  }
}

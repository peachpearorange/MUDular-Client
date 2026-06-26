use std::{fmt::Write, path::PathBuf};

#[cfg(not(target_arch = "wasm32"))]
use directories::ProjectDirs;

use crate::scripting::ScriptEngine;

#[derive(Clone, Debug)]
pub struct Profile {
  pub name: String,
  pub connection_mode: ConnectionMode,
  pub host: String,
  pub port: u16,
  pub tls: bool,
  pub websocket_url: Option<String>,
  pub script_code: String,
  pub path: Option<PathBuf>,
  pub is_preset: bool
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectionMode {
  Tcp,
  WebSocket
}

impl ConnectionMode {
  fn as_scheme_symbol(self) -> &'static str {
    match self {
      Self::Tcp => "tcp",
      Self::WebSocket => "websocket"
    }
  }
}

struct GaugeTemplate {
  name: &'static str,
  color: &'static str,
  gmcp_cur: &'static str,
  gmcp_max: &'static str
}

struct GameTemplate {
  name: &'static str,
  connection_mode: ConnectionMode,
  host: &'static str,
  port: u16,
  tls: bool,
  websocket_url: Option<&'static str>,
  has_map: bool,
  gauges: &'static [GaugeTemplate],
  gmcp_package: &'static str,
  extra_scheme: &'static str
}

const GAME_TEMPLATES: &[GameTemplate] = &[
  GameTemplate {
    name: "Achaea",
    connection_mode: ConnectionMode::Tcp,
    host: "achaea.com",
    port: 23,
    tls: false,
    websocket_url: None,
    has_map: true,
    gauges: &[
      GaugeTemplate { name: "health", color: "red", gmcp_cur: "hp", gmcp_max: "maxhp" },
      GaugeTemplate { name: "mana", color: "blue", gmcp_cur: "mp", gmcp_max: "maxmp" },
      GaugeTemplate {
        name: "endurance",
        color: "green",
        gmcp_cur: "ep",
        gmcp_max: "maxep"
      }
    ],
    gmcp_package: "Char.Vitals",
    extra_scheme: r#";; Route map-like lines (box-drawing borders) to the map pane
(define in-map #f)

(define (on-line line)
  (let ((text (strip-ansi line)))
    (cond
      ((or (regexp-match? "^\\s*--------\\+" text)
           (regexp-match? "^\\s*\\|.*\\|\\s*$" text))
       (set! in-map #t)
       (pane-print "map" line)
       #f)
      (in-map
       (if (or (equal? text "") (not (regexp-match? "[|\\-+]" text)))
           (begin (set! in-map #f) #t)
           (begin (pane-print "map" line) #f)))
      (else #t))))

;; Aliases
(alias "^gg$" (lambda ()
  (send "get gold from corpse")))

(alias "^aa (.+)$" (lambda (target)
  (send (to-string "attack " target))))
"#
  },
  GameTemplate {
    name: "Aardwolf",
    connection_mode: ConnectionMode::Tcp,
    host: "aardmud.org",
    port: 23,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[
      GaugeTemplate { name: "health", color: "red", gmcp_cur: "hp", gmcp_max: "maxhp" },
      GaugeTemplate {
        name: "mana",
        color: "blue",
        gmcp_cur: "mana",
        gmcp_max: "maxmana"
      },
      GaugeTemplate {
        name: "moves",
        color: "yellow",
        gmcp_cur: "moves",
        gmcp_max: "maxmoves"
      }
    ],
    gmcp_package: "char.vitals",
    extra_scheme: r#";; Aliases
(alias "^sc$" (lambda ()
  (send "score")))
"#
  },
  GameTemplate {
    name: "BatMUD",
    connection_mode: ConnectionMode::Tcp,
    host: "batmud.bat.org",
    port: 23,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[
      GaugeTemplate { name: "health", color: "red", gmcp_cur: "hp", gmcp_max: "maxhp" },
      GaugeTemplate { name: "sp", color: "blue", gmcp_cur: "sp", gmcp_max: "maxsp" },
      GaugeTemplate { name: "ep", color: "green", gmcp_cur: "ep", gmcp_max: "maxep" }
    ],
    gmcp_package: "Char.Vitals",
    extra_scheme: ""
  },
  GameTemplate {
    name: "Discworld",
    connection_mode: ConnectionMode::Tcp,
    host: "discworld.atuin.net",
    port: 4242,
    tls: false,
    websocket_url: None,
    has_map: true,
    gauges: &[
      GaugeTemplate { name: "hp", color: "red", gmcp_cur: "hp", gmcp_max: "maxhp" },
      GaugeTemplate { name: "gp", color: "blue", gmcp_cur: "gp", gmcp_max: "maxgp" },
      GaugeTemplate { name: "xp", color: "yellow", gmcp_cur: "", gmcp_max: "" }
    ],
    gmcp_package: "char.vitals",
    extra_scheme: r#";; Route map blocks (delimited by +---+) to the map pane
(define in-map #f)

(define (on-line line)
  (let ((text (strip-ansi line)))
    (cond
      ((regexp-match? "^\\+[-]+\\+$" text)
       (set! in-map (not in-map))
       (pane-print "map" line)
       #f)
      (in-map
       (pane-print "map" line)
       #f)
      (else #t))))

;; Aliases
(alias "^l$" (lambda ()
  (send "look")))
"#
  },
  GameTemplate {
    name: "GemStone IV",
    connection_mode: ConnectionMode::Tcp,
    host: "gemstone.net",
    port: 7777,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "DragonRealms",
    connection_mode: ConnectionMode::Tcp,
    host: "prime.dr.game.play.net",
    port: 4901,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: r#";; DragonRealms usually requires a time-sensitive Simutronics login key.
(define launch-key-note-shown #f)

(define (on-line line)
  (when (not launch-key-note-shown)
    (set! launch-key-note-shown #t)
    (pane-print "main" "[DragonRealms may need a launch key from the official Simutronics launcher.]"))
  #t)
"#
  },
  GameTemplate {
    name: "Threshold RPG",
    connection_mode: ConnectionMode::Tcp,
    host: "thresholdrpg.com",
    port: 3333,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "AwakeMUD CE",
    connection_mode: ConnectionMode::Tcp,
    host: "play.awakemud.com",
    port: 4000,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "Realms of Despair",
    connection_mode: ConnectionMode::Tcp,
    host: "realmsofdespair.com",
    port: 4000,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "Legends of the Jedi",
    connection_mode: ConnectionMode::Tcp,
    host: "legendsofthejedi.com",
    port: 5656,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "Miriani",
    connection_mode: ConnectionMode::Tcp,
    host: "toastsoft.net",
    port: 1234,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "Alter Aeon",
    connection_mode: ConnectionMode::Tcp,
    host: "alteraeon.com",
    port: 3000,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "Genesis",
    connection_mode: ConnectionMode::Tcp,
    host: "mud.genesismud.org",
    port: 3011,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "The Eternal City",
    connection_mode: ConnectionMode::Tcp,
    host: "game.eternalcitygame.com",
    port: 6730,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "Materia Magica",
    connection_mode: ConnectionMode::Tcp,
    host: "materiamagica.com",
    port: 23,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "Avatar MUD",
    connection_mode: ConnectionMode::Tcp,
    host: "avatar.outland.org",
    port: 3000,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "Star Trek: Phoenix Rising",
    connection_mode: ConnectionMode::Tcp,
    host: "game.phxrising.org",
    port: 1701,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "CLOK",
    connection_mode: ConnectionMode::Tcp,
    host: "clok.contrarium.net",
    port: 4000,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "CoffeeMud",
    connection_mode: ConnectionMode::Tcp,
    host: "coffeemud.net",
    port: 23,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "MUME",
    connection_mode: ConnectionMode::Tcp,
    host: "mume.org",
    port: 4242,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "Icesus MUD",
    connection_mode: ConnectionMode::Tcp,
    host: "icesus.org",
    port: 4000,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "Dune",
    connection_mode: ConnectionMode::Tcp,
    host: "dunemud.net",
    port: 6789,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "Merentha",
    connection_mode: ConnectionMode::Tcp,
    host: "merentha.com",
    port: 10000,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "Lost Souls",
    connection_mode: ConnectionMode::Tcp,
    host: "lostsouls.org",
    port: 23,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "RetroMUD",
    connection_mode: ConnectionMode::Tcp,
    host: "retromud.org",
    port: 3000,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "Mirkwood",
    connection_mode: ConnectionMode::Tcp,
    host: "mirkwoodmud.org",
    port: 4000,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "Enrym TCP",
    connection_mode: ConnectionMode::Tcp,
    host: "play.enrym.com",
    port: 4001,
    tls: true,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: r#";; Log GMCP messages
(define (on-gmcp package data)
  (pane-print "main" (to-string "[GMCP " package "]")))
"#
  },
  GameTemplate {
    name: "Enrym WebSocket",
    connection_mode: ConnectionMode::WebSocket,
    host: "play.enrym.com",
    port: 4001,
    tls: true,
    websocket_url: Some("wss://play.enrym.com"),
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: r#";; Log GMCP messages
(define (on-gmcp package data)
  (pane-print "main" (to-string "[GMCP " package "]")))
"#
  },
  GameTemplate {
    name: "Generic",
    connection_mode: ConnectionMode::Tcp,
    host: "localhost",
    port: 4000,
    tls: false,
    websocket_url: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: r#";; Log GMCP messages
(define (on-gmcp package data)
  (pane-print "main" (to-string "[GMCP " package "]")))
"#
  }
];

fn generate_scheme(t: &GameTemplate) -> String {
  let mut s = String::new();

  let _ = writeln!(s, "(profile");
  let _ = writeln!(s, "  'name \"{}\"", t.name);
  let _ = writeln!(s, "  'connection-mode '{}", t.connection_mode.as_scheme_symbol());
  let _ = writeln!(s, "  'host \"{}\"", t.host);
  let _ = writeln!(s, "  'port {}", t.port);
  let _ = write!(s, "  'tls {}", if t.tls { "#t" } else { "#f" });
  if let Some(url) = t.websocket_url {
    let _ = writeln!(s);
    let _ = write!(s, "  'websocket-url \"{url}\"");
  }
  let _ = writeln!(s, ")");
  let _ = writeln!(s);
  let _ = writeln!(
    s,
    ";; You can use any of 550+ built-in themes from https://iterm2colorschemes.com"
  );
  let _ = writeln!(s, "(load-theme \"Onenord\")");
  let _ = writeln!(s, "(option \"scroll_lines\" 6)");
  let _ = writeln!(s);
  let _ = writeln!(s, ";; Scrolling");
  let _ = writeln!(s, "(keymap \"PageUp\" \"scroll_up 20\")");
  let _ = writeln!(s, "(keymap \"PageDown\" \"scroll_down 20\")");
  let _ = writeln!(s);
  let _ = writeln!(s, ";; Panes");
  let _ = writeln!(s, "(pane \"main\")");

  if t.has_map {
    let _ = writeln!(s, "(pane \"map\")");
    let _ = writeln!(s);
    let _ = writeln!(s, "(layout \"horizontal\" (list");
    let _ = writeln!(s, "    (list \"main\" 3)");
    let _ = writeln!(s, "    (list \"map\" 1)))");
  }
  let _ = writeln!(s);

  for g in t.gauges {
    let _ = writeln!(s, "(gauge \"{}\" (hash 'color \"{}\"))", g.name, g.color);
  }
  let _ = writeln!(s);

  let _ = writeln!(s, "(define (on-connect)");
  let _ = writeln!(s, "  (pane-print \"main\" \"[Connected to {}]\"))", t.name);
  let _ = writeln!(s);
  let _ = writeln!(s, "(define (on-disconnect)");
  let _ = writeln!(s, "  (pane-print \"main\" \"[Disconnected from {}]\"))", t.name);

  if !t.extra_scheme.contains("(define (on-line") {
    let _ = writeln!(s);
    let _ = writeln!(s, "(define (on-line line) #t)");
  }

  if !t.extra_scheme.contains("(define (on-gmcp") && !t.gmcp_package.is_empty() {
    let _ = writeln!(s);
    let _ = writeln!(s, "(define (on-gmcp package data)");
    let _ = writeln!(s, "  (when (equal? package \"{}\")", t.gmcp_package);
    for g in t.gauges {
      if !g.gmcp_cur.is_empty() {
        let _ = writeln!(
          s,
          "    (when (and (hash-contains? data \"{}\") (hash-contains? data \"{}\"))",
          g.gmcp_cur, g.gmcp_max
        );
        let _ = writeln!(
          s,
          "      (gauge \"{}\" (hash 'current (hash-ref data \"{}\") 'max (hash-ref data \"{}\") 'color \"{}\")))",
          g.name, g.gmcp_cur, g.gmcp_max, g.color,
        );
      }
    }
    let _ = writeln!(s, "    ))");
  }

  if !t.extra_scheme.is_empty() {
    let _ = writeln!(s);
    s.push_str(t.extra_scheme);
  }

  s
}

impl Profile {
  pub fn profiles_dir() -> Option<PathBuf> {
    #[cfg(not(target_arch = "wasm32"))]
    {
      ProjectDirs::from("com", "mudular", "mudular-client")
        .map(|dirs| dirs.config_dir().join("profiles"))
    }
    #[cfg(target_arch = "wasm32")]
    None
  }

  pub fn load_user() -> Vec<Profile> {
    #[cfg(not(target_arch = "wasm32"))]
    {
      Self::load_user_profiles()
    }
    #[cfg(target_arch = "wasm32")]
    Vec::new()
  }

  pub fn templates() -> Vec<Profile> {
    let mut templates: Vec<Profile> = GAME_TEMPLATES
      .iter()
      .map(|t| Profile {
        name: t.name.into(),
        connection_mode: t.connection_mode,
        host: t.host.into(),
        port: t.port,
        tls: t.tls,
        websocket_url: t.websocket_url.map(str::to_string),
        script_code: generate_scheme(t),
        path: None,
        is_preset: true
      })
      .collect();
    templates.push(Profile {
      name: "NukeFire".into(),
      connection_mode: ConnectionMode::Tcp,
      host: "tdome.nukefire.org".into(),
      port: 4000,
      tls: false,
      websocket_url: None,
      script_code: include_str!("../profiles/nukefire/init.scm").into(),
      path: None,
      is_preset: true
    });
    templates
  }

  pub fn unique_name(base: &str, existing: &[Profile]) -> String {
    if !existing.iter().any(|p| p.name == base) {
      base.to_string()
    } else {
      (2..)
        .map(|i| format!("{base}_{i}"))
        .find(|candidate| !existing.iter().any(|p| p.name == *candidate))
        .unwrap()
    }
  }

  fn load_user_profiles() -> Vec<Profile> {
    let Some(dir) = Self::profiles_dir() else {
      return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
      return Vec::new();
    };

    entries
      .filter_map(|e| e.ok())
      .filter(|e| e.path().is_dir())
      .filter_map(|e| {
        let scm_path = e.path().join("init.scm");
        let lua_path = e.path().join("init.lua");
        let (path, code) = std::fs::read_to_string(&scm_path)
          .ok()
          .map(|c| (scm_path, c))
          .or_else(|| std::fs::read_to_string(&lua_path).ok().map(|c| (lua_path, c)))?;
        let fallback_name = e.file_name().to_string_lossy().to_string();
        let metadata = load_profile_metadata(&code);
        Some(Profile {
          name: metadata.name.unwrap_or_else(|| fallback_name.clone()),
          connection_mode: metadata.connection_mode.unwrap_or(ConnectionMode::Tcp),
          host: metadata.host.unwrap_or_else(|| "localhost".into()),
          port: metadata.port.unwrap_or(4000),
          tls: metadata.tls.unwrap_or(false),
          websocket_url: metadata.websocket_url,
          script_code: code,
          path: Some(path),
          is_preset: false
        })
      })
      .collect()
  }

  pub fn save(&mut self) -> Result<(), String> {
    let dir = Self::profiles_dir().ok_or("Could not determine config directory")?;
    let profile_dir = dir.join(&self.name);
    std::fs::create_dir_all(&profile_dir).map_err(|e| e.to_string())?;
    let scm_path = profile_dir.join("init.scm");
    std::fs::write(&scm_path, &self.script_code).map_err(|e| e.to_string())?;
    self.path = Some(scm_path);
    self.is_preset = false;
    Ok(())
  }

  pub fn delete(&self) -> Result<(), String> {
    let path = self.path.as_ref().ok_or("No path for this profile")?;
    let dir = path.parent().ok_or("Invalid path")?;
    std::fs::remove_dir_all(dir).map_err(|e| e.to_string())
  }

  pub fn rename(&mut self, new_name: &str) -> Result<(), String> {
    let dir = Self::profiles_dir().ok_or("Could not determine config directory")?;
    let old_dir = dir.join(&self.name);
    let new_dir = dir.join(new_name);
    if old_dir.exists() {
      std::fs::rename(&old_dir, &new_dir).map_err(|e| e.to_string())?;
    }
    let old_name_line = format!("  'name \"{}\"", self.name);
    let new_name_line = format!("  'name \"{}\"", new_name);
    self.script_code = self.script_code.replacen(&old_name_line, &new_name_line, 1);
    self.name = new_name.to_string();
    let scm_path = new_dir.join("init.scm");
    std::fs::write(&scm_path, &self.script_code).map_err(|e| e.to_string())?;
    self.path = Some(scm_path);
    Ok(())
  }
}

#[derive(Default)]
struct ProfileMetadata {
  name: Option<String>,
  connection_mode: Option<ConnectionMode>,
  host: Option<String>,
  port: Option<u16>,
  tls: Option<bool>,
  websocket_url: Option<String>
}

fn load_profile_metadata(code: &str) -> ProfileMetadata {
  let Ok(mut engine) = ScriptEngine::new() else {
    return ProfileMetadata::default();
  };
  if engine.load_script(code).is_err() {
    return ProfileMetadata::default();
  }
  let st = engine.state.lock().unwrap();
  ProfileMetadata {
    name: st.profile_name.clone(),
    connection_mode: st.profile_connection_mode.as_deref().and_then(|mode| match mode {
      "tcp" => Some(ConnectionMode::Tcp),
      "websocket" => Some(ConnectionMode::WebSocket),
      _ => None
    }),
    host: st.profile_host.clone(),
    port: st.profile_port,
    tls: st.profile_tls,
    websocket_url: st.profile_websocket_url.clone()
  }
}

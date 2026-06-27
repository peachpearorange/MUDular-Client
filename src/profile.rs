use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
use directories::ProjectDirs;

#[derive(Clone, Debug)]
pub struct Profile {
  pub name: String,
  pub connection_mode: ConnectionMode,
  pub host: String,
  pub port: u16,
  pub tls: bool,
  pub websocket_url: Option<String>,
  pub websocket_protocol: Option<String>,
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
  websocket_protocol: Option<&'static str>,
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
    websocket_protocol: None,
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

(mud/on "line" (lambda (line)
  (let ((text (mud/strip-ansi line)))
    (cond
      ((or (mud/regexp-match? "^\\s*--------\\+" text)
           (mud/regexp-match? "^\\s*\\|.*\\|\\s*$" text))
       (set! in-map #t)
       (mud/pane-print "map" line)
       #f)
      (in-map
       (if (or (equal? text "") (not (mud/regexp-match? "[|\\-+]" text)))
           (begin (set! in-map #f) #t)
           (begin (mud/pane-print "map" line) #f)))
      (else #t)))))

;; Aliases
(alias "^gg$" (lambda ()
  (mud/send "get gold from corpse")))

(alias "^aa (.+)$" (lambda (target)
  (mud/send (to-string "attack " target))))
"#
  },
  GameTemplate {
    name: "Aardwolf",
    connection_mode: ConnectionMode::Tcp,
    host: "aardmud.org",
    port: 23,
    tls: false,
    websocket_url: None,
    websocket_protocol: None,
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
  (mud/send "score")))
"#
  },
  GameTemplate {
    name: "BatMUD",
    connection_mode: ConnectionMode::Tcp,
    host: "batmud.bat.org",
    port: 23,
    tls: false,
    websocket_url: None,
    websocket_protocol: None,
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
    websocket_protocol: None,
    has_map: true,
    gauges: &[
      GaugeTemplate { name: "hp", color: "red", gmcp_cur: "hp", gmcp_max: "maxhp" },
      GaugeTemplate { name: "gp", color: "blue", gmcp_cur: "gp", gmcp_max: "maxgp" },
      GaugeTemplate { name: "xp", color: "yellow", gmcp_cur: "", gmcp_max: "" }
    ],
    gmcp_package: "char.vitals",
    extra_scheme: r#";; Route map blocks (delimited by +---+) to the map pane
(define in-map #f)

(mud/on "line" (lambda (line)
  (let ((text (mud/strip-ansi line)))
    (cond
      ((mud/regexp-match? "^\\+[-]+\\+$" text)
       (set! in-map (not in-map))
       (mud/pane-print "map" line)
       #f)
      (in-map
       (mud/pane-print "map" line)
       #f)
      (else #t)))))

;; Aliases
(alias "^l$" (lambda ()
  (mud/send "look")))
"#
  },
  GameTemplate {
    name: "GemStone IV",
    connection_mode: ConnectionMode::Tcp,
    host: "gemstone.net",
    port: 7777,
    tls: false,
    websocket_url: None,
    websocket_protocol: None,
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
    websocket_protocol: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: r#";; DragonRealms usually requires a time-sensitive Simutronics login key.
(define launch-key-note-shown #f)

(mud/on "line" (lambda (line)
  (when (not launch-key-note-shown)
    (set! launch-key-note-shown #t)
    (mud/pane-print "main" "[DragonRealms may need a launch key from the official Simutronics launcher.]"))
  #t))
"#
  },
  GameTemplate {
    name: "Threshold RPG",
    connection_mode: ConnectionMode::Tcp,
    host: "thresholdrpg.com",
    port: 3333,
    tls: false,
    websocket_url: None,
    websocket_protocol: None,
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
    websocket_protocol: None,
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
    websocket_protocol: None,
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
    websocket_protocol: None,
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
    websocket_protocol: None,
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
    websocket_protocol: None,
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
    websocket_protocol: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "Genesis WebSocket (experimental)",
    connection_mode: ConnectionMode::WebSocket,
    host: "mud.genesismud.org",
    port: 3011,
    tls: false,
    websocket_url: Some("wss://www.genesismud.org/websocket"),
    websocket_protocol: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: r#";; Genesis documents this WebSocket endpoint for its official web client.
"#
  },
  GameTemplate {
    name: "The Eternal City",
    connection_mode: ConnectionMode::Tcp,
    host: "game.eternalcitygame.com",
    port: 6730,
    tls: false,
    websocket_url: None,
    websocket_protocol: None,
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
    websocket_protocol: None,
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
    websocket_protocol: None,
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
    websocket_protocol: None,
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
    websocket_protocol: None,
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
    websocket_protocol: None,
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
    websocket_protocol: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: ""
  },
  GameTemplate {
    name: "MUME WebSocket (experimental)",
    connection_mode: ConnectionMode::WebSocket,
    host: "mume.org",
    port: 4242,
    tls: false,
    websocket_url: Some("wss://mume.org/ws-play/"),
    websocket_protocol: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: r#";; Experimental: MUME documents an official WebSocket proxy at this path.
"#
  },
  GameTemplate {
    name: "Icesus MUD",
    connection_mode: ConnectionMode::Tcp,
    host: "icesus.org",
    port: 4000,
    tls: false,
    websocket_url: None,
    websocket_protocol: None,
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
    websocket_protocol: None,
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
    websocket_protocol: None,
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
    websocket_protocol: None,
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
    websocket_protocol: None,
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
    websocket_protocol: None,
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
    websocket_protocol: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: r#";; Log GMCP messages
(mud/on "gmcp" (lambda (package data)
  (mud/pane-print "main" (to-string "[GMCP " package "]"))))
"#
  },
  GameTemplate {
    name: "Enrym WebSocket",
    connection_mode: ConnectionMode::WebSocket,
    host: "play.enrym.com",
    port: 4001,
    tls: true,
    websocket_url: Some("wss://play.enrym.com"),
    websocket_protocol: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: r#";; Log GMCP messages
(mud/on "gmcp" (lambda (package data)
  (mud/pane-print "main" (to-string "[GMCP " package "]"))))
"#
  },
  GameTemplate {
    name: "Generic",
    connection_mode: ConnectionMode::Tcp,
    host: "localhost",
    port: 4000,
    tls: false,
    websocket_url: None,
    websocket_protocol: None,
    has_map: false,
    gauges: &[],
    gmcp_package: "",
    extra_scheme: r#";; Log GMCP messages
(mud/on "gmcp" (lambda (package data)
  (mud/pane-print "main" (to-string "[GMCP " package "]"))))
"#
  }
];

fn generate_scheme(t: &GameTemplate) -> String {
  let tls = if t.tls { "#t" } else { "#f" };

  let websocket = t
    .websocket_url
    .map(|url| format!("\n  'websocket-url \"{url}\""))
    .unwrap_or_default();
  let websocket_protocol = t
    .websocket_protocol
    .map(|protocol| format!("\n  'websocket-protocol \"{protocol}\""))
    .unwrap_or_default();

  let map = if t.has_map {
    "(mud/pane \"map\")\n\n\
     (mud/layout \"horizontal\" (list\n\
     \x20   (list \"main\" 3)\n\
     \x20   (list \"map\" 1)))\n"
  } else {
    ""
  };

  let gauges: String = t
    .gauges
    .iter()
    .map(|g| format!("(mud/gauge \"{}\" (hash 'color \"{}\"))\n", g.name, g.color))
    .collect();

  let gmcp = if t.gmcp_package.is_empty() {
    String::new()
  } else {
    let handlers: String = t.gauges.iter()
      .filter(|g| !g.gmcp_cur.is_empty())
      .map(|g| format!(
        "    (when (and (hash-contains? data \"{cur}\") (hash-contains? data \"{max}\"))\n\
         \x20     (mud/gauge \"{name}\" (hash 'current (hash-ref data \"{cur}\") \
                   'max (hash-ref data \"{max}\") 'color \"{color}\")))\n",
        cur = g.gmcp_cur, max = g.gmcp_max, name = g.name, color = g.color
      ))
      .collect();
    format!(
      "\n(mud/on \"gmcp\" (lambda (package data)\n\
       \x20 (when (equal? package \"{pkg}\")\n\
       {handlers}\
       \x20   )))\n",
      pkg = t.gmcp_package
    )
  };

  let extra = if t.extra_scheme.is_empty() {
    String::new()
  } else {
    format!("\n{}", t.extra_scheme)
  };

  format!(
    "\
;; Steel implementation of R5RS Scheme

(mud/profile
  'name \"{name}\"
  'connection-mode '{mode}
  'host \"{host}\"
  'port {port}
  'tls {tls}{websocket}{websocket_protocol})

;; Enter your character and password here to log in automatically on connect.
;; Leave empty to log in manually.
(define character \"\")
(define password \"\")

;; Use /(mud/themes) to see available color schemes.
(mud/load-theme \"Onenord\")
;; Use /(mud/fonts) to see available fonts.
;; (mud/option \"font\" \"JetBrains Mono\")
(mud/option \"font_size\" 14)
(mud/option \"scroll_lines\" 6)

;; Scrolling
(mud/keymap \"PageUp\" \"scroll_up 20\")
(mud/keymap \"PageDown\" \"scroll_down 20\")

;; Panes
(mud/pane \"main\")
{map}
{gauges}
(mud/on \"connect\" (lambda ()
  (mud/pane-print \"main\" \"[Connected to {name}]\")
  (when (not (equal? character \"\"))
    (timer 0.5 (lambda () (mud/send character)))
    (timer 1.0 (lambda () (mud/send password))))))

(mud/on \"disconnect\" (lambda ()
  (mud/pane-print \"main\" \"[Disconnected from {name}]\")))
{gmcp}{extra}",
    name = t.name,
    mode = t.connection_mode.as_scheme_symbol(),
    host = t.host,
    port = t.port,
  )
}

struct ScriptedTemplate {
  name: &'static str,
  connection_mode: ConnectionMode,
  host: &'static str,
  port: u16,
  tls: bool,
  websocket_url: Option<&'static str>,
  websocket_protocol: Option<&'static str>,
  script: &'static str
}

const SCRIPTED_TEMPLATES: &[ScriptedTemplate] = &[
  ScriptedTemplate {
    name: "NukeFire",
    connection_mode: ConnectionMode::Tcp,
    host: "tdome.nukefire.org",
    port: 4000,
    tls: false,
    websocket_url: None,
    websocket_protocol: None,
    script: include_str!("../profiles/nukefire/init.scm")
  },
  ScriptedTemplate {
    name: "NukeFire WebSocket (experimental)",
    connection_mode: ConnectionMode::WebSocket,
    host: "tintin.nukefire.org",
    port: 443,
    tls: false,
    websocket_url: Some("wss://tintin.nukefire.org/ws"),
    websocket_protocol: Some("tty"),
    script: include_str!("../profiles/nukefire-ws/init.scm")
  }
];

fn scripted_templates() -> impl Iterator<Item = Profile> {
  SCRIPTED_TEMPLATES.iter().map(
    |&ScriptedTemplate {
       name,
       connection_mode,
       host,
       port,
       tls,
       websocket_url,
       websocket_protocol,
       script
     }| Profile {
      name: name.into(),
      connection_mode,
      host: host.into(),
      port,
      tls,
      websocket_url: websocket_url.map(str::to_string),
      websocket_protocol: websocket_protocol.map(str::to_string),
      script_code: script.into(),
      path: None,
      is_preset: true
    }
  )
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
    GAME_TEMPLATES
      .iter()
      .map(|t| Profile {
        name: t.name.into(),
        connection_mode: t.connection_mode,
        host: t.host.into(),
        port: t.port,
        tls: t.tls,
        websocket_url: t.websocket_url.map(str::to_string),
        websocket_protocol: t.websocket_protocol.map(str::to_string),
        script_code: generate_scheme(t),
        path: None,
        is_preset: true
      })
      .chain(scripted_templates())
      .collect()
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
    Self::profiles_dir()
      .and_then(|dir| std::fs::read_dir(&dir).ok())
      .into_iter()
      .flatten()
      .filter_map(|e| e.ok())
      .filter(|e| e.path().is_dir())
      .filter_map(|e| {
        let scm_path = e.path().join("init.scm");
        let (path, code) =
          std::fs::read_to_string(&scm_path).ok().map(|c| (scm_path, c))?;
        let fallback_name = e.file_name().to_string_lossy().to_string();
        let metadata = parse_profile_metadata(&code);
        Some(Profile {
          name: metadata.name.unwrap_or_else(|| fallback_name.clone()),
          connection_mode: metadata.connection_mode.unwrap_or(ConnectionMode::Tcp),
          host: metadata.host.unwrap_or_else(|| "localhost".into()),
          port: metadata.port.unwrap_or(4000),
          tls: metadata.tls.unwrap_or(false),
          websocket_url: metadata.websocket_url,
          websocket_protocol: metadata.websocket_protocol,
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
  websocket_url: Option<String>,
  websocket_protocol: Option<String>
}

fn parse_profile_metadata(code: &str) -> ProfileMetadata {
  let mut meta = ProfileMetadata::default();
  for line in code.lines().map(str::trim) {
    if let Some(val) = line.strip_prefix("'name \"").and_then(|s| s.strip_suffix('"')) {
      meta.name = Some(val.to_string());
    } else if let Some(val) =
      line.strip_prefix("'host \"").and_then(|s| s.strip_suffix('"'))
    {
      meta.host = Some(val.to_string());
    } else if let Some(val) = line.strip_prefix("'port ") {
      meta.port = val.trim_end_matches(')').trim().parse().ok();
    } else if let Some(val) = line.strip_prefix("'tls ") {
      meta.tls = Some(val.trim_end_matches(')').trim() == "#t");
    } else if let Some(val) = line.strip_prefix("'connection-mode '") {
      meta.connection_mode = match val.trim_end_matches(')').trim() {
        "tcp" => Some(ConnectionMode::Tcp),
        "websocket" => Some(ConnectionMode::WebSocket),
        _ => None
      };
    } else if let Some(val) =
      line.strip_prefix("'websocket-url \"").and_then(|s| s.strip_suffix("\")"))
    {
      meta.websocket_url = Some(val.to_string());
    } else if let Some(val) =
      line.strip_prefix("'websocket-protocol \"").and_then(|s| s.strip_suffix("\")"))
    {
      meta.websocket_protocol = Some(val.to_string());
    }
  }
  meta
}

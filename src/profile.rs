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
  name: String,
  connection_mode: ConnectionMode,
  host: String,
  port: u16,
  tls: bool,
  websocket_url: Option<String>,
  websocket_protocol: Option<String>,
  scheme: String
}

impl GameTemplate {
  fn new(
    name: impl Into<String>,
    connection_mode: ConnectionMode,
    host: impl Into<String>,
    port: u16,
    tls: bool
  ) -> Self {
    Self {
      name: name.into(),
      connection_mode,
      host: host.into(),
      port,
      tls,
      websocket_url: None,
      websocket_protocol: None,
      scheme: String::new()
    }
  }

  fn websocket(mut self, url: impl Into<String>, protocol: Option<String>) -> Self {
    self.websocket_url = Some(url.into());
    self.websocket_protocol = protocol;
    self
  }

  /// Append raw scheme text to the body (between the standard header/keybinds and
  /// nothing — the template composes panes, gauges, hooks, aliases here).
  fn concat(mut self, text: impl Into<String>) -> Self {
    self.scheme.push_str(&text.into());
    self
  }

  /// Set the theme. Concats a `(mud/set-theme ...)` call; OPTIONS_BLOCK emits no
  /// theme, so this is the single theme load.
  fn theme(self, name: &str) -> Self {
    self.concat(format!("(mud/set-theme {name})\n"))
  }

  /// Enable keep-input (don't clear the input line on submit).
  fn keep_input(self) -> Self {
    self.concat("(mud/set-keep-input #t)\n")
  }

  /// Apply the shared NukeFire game configuration: theme, keep-input, movement
  /// keymaps, the map pane with a 1:1 main/map layout, the three gauges, and the
  /// MSDP/map-parser/connect hooks. Used by both the TCP and WebSocket variants.
  fn apply_nukefire_config(self) -> Self {
    self
      .theme("theme/toy-chest")
      .keep_input()
      .concat(
        ";; Movement: Alt+WASD + Alt+Q/E\n\
         (mud/keymap \"alt+w\" (lambda () (mud/send \"n\")))\n\
         (mud/keymap \"alt+s\" (lambda () (mud/send \"s\")))\n\
         (mud/keymap \"alt+a\" (lambda () (mud/send \"w\")))\n\
         (mud/keymap \"alt+d\" (lambda () (mud/send \"e\")))\n\
         (mud/keymap \"alt+q\" (lambda () (mud/send \"d\")))\n\
         (mud/keymap \"alt+e\" (lambda () (mud/send \"u\")))\n"
      )
      .concat(
        "\n\
         (mud/pane \"map\")\n\
         (mud/layout \"horizontal\" (list\n\
             (list \"main\" 1)\n\
             (list \"map\" 1)))\n"
      )
      .gauges(&[
        gauge("health", "green", "", ""),
        gauge("mana", "cyan", "", ""),
        gauge("moves", "blue", "", "")
      ])
      .concat(nukefire_custom_block())
  }

  /// Append the map pane plus a horizontal main/map layout.
  fn map_panes(self) -> Self {
    self.concat(
      "\n(mud/pane \"map\")\n\
       (mud/layout \"horizontal\" (list\n\
       \x20   (list \"main\" 3)\n\
       \x20   (list \"map\" 1)))\n"
    )
  }

  /// Append gauge declarations.
  fn gauges(self, gauges: &[GaugeTemplate]) -> Self {
    self.concat(gauges_block(gauges))
  }

  /// Append the generated GMCP gauge-update hook (no-op if no gauge has gmcp fields).
  fn gmcp(self, package: &str, gauges: &[GaugeTemplate]) -> Self {
    self.concat(gmcp_gauge_hook(package, gauges))
  }

  /// Append the standard connect/disconnect hooks (uses the profile name).
  fn connect(self) -> Self {
    let name = self.name.clone();
    self.concat(connect_block(&name))
  }

  /// Append the three default event hooks (on-line, on-input, on-msdp) as no-ops.
  /// Use `default_on_gmcp()` or `gmcp()` for the GMCP hook, since it's commonly
  /// customized.
  fn default_hooks(self) -> Self {
    self.concat(DEFAULT_HOOKS)
  }

  /// Append the default no-op on-gmcp hook.
  fn default_on_gmcp(self) -> Self {
    self.concat(DEFAULT_ON_GMCP)
  }

  /// Append a custom on-line hook (replaces the default from `default_hooks`).
  fn on_line(self, body: impl Into<String>) -> Self {
    let body = body.into();
    self.concat(format!(
      ";; Fired for each line received from the server. Return #f to suppress it.\n\
       (mud/on-line {body})\n"
    ))
  }

  /// Append a custom on-gmcp hook (replaces the default / generated one).
  fn on_gmcp(self, body: impl Into<String>) -> Self {
    let body = body.into();
    self.concat(format!(
      ";; Fired when the server sends a GMCP message. data is a hash.\n\
       (mud/on-gmcp {body})\n"
    ))
  }

  /// Append a custom on-msdp hook.
  #[allow(dead_code)]
  fn on_msdp(self, body: impl Into<String>) -> Self {
    let body = body.into();
    self.concat(format!(
      ";; Fired when the server sends an MSDP message. data is a hash.\n\
       (mud/on-msdp {body})\n"
    ))
  }

  /// Append a custom on-input hook.
  #[allow(dead_code)]
  fn on_input(self, body: impl Into<String>) -> Self {
    let body = body.into();
    self.concat(format!(
      ";; Fired for each command you enter (before aliases/triggers).\n\
       (mud/on-input {body})\n"
    ))
  }

  fn build(self) -> Profile {
    let mut s = String::new();
    s.push_str(";; Steel implementation of R5RS Scheme\n\n");
    s.push_str(&self.header());
    s.push_str(";; Enter your character and password here to log in automatically on connect.\n");
    s.push_str(";; Leave empty to log in manually.\n");
    s.push_str("(define character \"\")\n(define password \"\")\n\n");
    s.push_str(OPTIONS_BLOCK_PREFIX);
    s.push_str(&format!(
      ";; Discord Rich Presence\n\
       (mud/discord-rpc \"Playing {}\")\n\n",
      self.name.split(" WebSocket").next().unwrap_or(&self.name)
    ));
    s.push_str(DEFAULT_KEYBINDS);
    s.push_str("\n;; Panes\n(mud/pane \"main\")\n");
    s.push_str(&self.scheme);
    Profile {
      name: self.name,
      connection_mode: self.connection_mode,
      host: self.host,
      port: self.port,
      tls: self.tls,
      websocket_url: self.websocket_url,
      websocket_protocol: self.websocket_protocol,
      script_code: s,
      path: None,
      is_preset: true
    }
  }

  fn header(&self) -> String {
    let tls = if self.tls { "#t" } else { "#f" };
    let websocket = self
      .websocket_url
      .as_deref()
      .map(|url| format!("\n  'websocket-url \"{url}\""))
      .unwrap_or_default();
    let websocket_protocol = self
      .websocket_protocol
      .as_deref()
      .map(|protocol| format!("\n  'websocket-protocol \"{protocol}\""))
      .unwrap_or_default();
    format!(
      "(mud/profile\n  'connection-mode '{mode}\n  \
       'host \"{host}\"\n  'port {port}\n  'tls {tls}{websocket}{websocket_protocol})\n\n",
      mode = self.connection_mode.as_scheme_symbol(),
      host = self.host,
      port = self.port
    )
  }
}

fn gauge(name: &'static str, color: &'static str, gmcp_cur: &'static str, gmcp_max: &'static str) -> GaugeTemplate {
  GaugeTemplate { name, color, gmcp_cur, gmcp_max }
}

/// `(mud/on-connect ...)` + `(mud/on-disconnect ...)` with the character/password
/// auto-login timers.
fn connect_block(name: &str) -> String {
  format!(
    "\n(mud/on-connect (lambda ()\n  (mud/pane-print \"main\" \"[Connected to {name}]\")\n  \
     (when (not (equal? character \"\"))\n    (mud/timer 0.5 (lambda () (mud/send character)))\n    \
     (mud/timer 1.0 (lambda () (mud/send password))))))\n\n\
     (mud/on-disconnect (lambda ()\n  (mud/pane-print \"main\" \"[Disconnected from {name}]\")))\n"
  )
}

/// Gauge declarations: `(mud/gauge "name" (hash 'color "color"))` per gauge.
fn gauges_block(gauges: &[GaugeTemplate]) -> String {
  gauges
    .iter()
    .map(|g| format!("(mud/gauge \"{}\" (hash 'color \"{}\"))\n", g.name, g.color))
    .collect()
}

/// Generated on-gmcp hook that updates gauges from a `Char.Vitals`-style package.
/// Empty string unless at least one gauge has gmcp fields.
fn gmcp_gauge_hook(package: &str, gauges: &[GaugeTemplate]) -> String {
  let handlers: String = gauges
    .iter()
    .filter(|g| !g.gmcp_cur.is_empty())
    .map(|g| {
      format!(
        "    (when (and (hash-contains? data \"{cur}\") (hash-contains? data \"{max}\"))\n\
         \x20     (mud/gauge \"{name}\" (hash 'current (hash-ref data \"{cur}\") \
                   'max (hash-ref data \"{max}\") 'color \"{color}\")))\n",
        cur = g.gmcp_cur, max = g.gmcp_max, name = g.name, color = g.color
      )
    })
    .collect();
  if handlers.is_empty() {
    return String::new();
  }
  format!(
    "\n;; Fired when the server sends a GMCP message. data is a hash.\n\
     (mud/on-gmcp (lambda (package data)\n  (when (equal? package \"{pkg}\")\n{handlers}    )))\n",
    pkg = package
  )
}

fn nukefire_custom_block() -> &'static str {
  include_str!("profiles/nukefire_custom.scm")
}

const OPTIONS_BLOCK_PREFIX: &str = "\
;; Use /(mud/themes) to see available color schemes.\n\
;; Use /(mud/fonts) to see available fonts.\n\
;; (mud/set-font \"JetBrains Mono\")\n\
(mud/set-font-size 14)\n\
(mud/set-scroll-lines 6)\n";

const DEFAULT_KEYBINDS: &str = "\
;; Scrolling\n\
(mud/keymap \"PageUp\" (lambda () (mud/scroll-up 20)))\n\
(mud/keymap \"PageDown\" (lambda () (mud/scroll-down 20)))\n\
\n\
;; Font size\n\
(mud/keymap \"alt+plus\" (lambda () (mud/pane-print \"main\" (to-string \"[Font size: \" (mud/increase-font-size) \"]\"))))\n\
(mud/keymap \"alt+minus\" (lambda () (mud/pane-print \"main\" (to-string \"[Font size: \" (mud/decrease-font-size) \"]\"))))\n\
\n\
;; Reconnect\n\
(mud/keymap \"alt+r\" (lambda () (mud/reconnect)))\n\
\n\
;; Key combo capture\n\
(mud/keymap \"alt+k\" (lambda () (mud/capture-key)))\n\
\n\
;; Instant number sending\n\
(mud/keymap \"alt+0\" (lambda () (mud/send \"0\")))\n\
(mud/keymap \"alt+1\" (lambda () (mud/send \"1\")))\n\
(mud/keymap \"alt+2\" (lambda () (mud/send \"2\")))\n\
(mud/keymap \"alt+3\" (lambda () (mud/send \"3\")))\n\
(mud/keymap \"alt+4\" (lambda () (mud/send \"4\")))\n\
(mud/keymap \"alt+5\" (lambda () (mud/send \"5\")))\n\
(mud/keymap \"alt+6\" (lambda () (mud/send \"6\")))\n\
(mud/keymap \"alt+7\" (lambda () (mud/send \"7\")))\n\
(mud/keymap \"alt+8\" (lambda () (mud/send \"8\")))\n\
(mud/keymap \"alt+9\" (lambda () (mud/send \"9\")))\n";

/// Default on-line, on-input, on-msdp no-op hooks. Each hook is a single
/// definition with a commented-out debug print line inside it — uncomment
/// the body line to see incoming data. on-gmcp is separate — see
/// `default_on_gmcp` / `gmcp`.
const DEFAULT_HOOKS: &str = r##";; Fired for each line received from the server. Return #f to suppress it.
(mud/on-line (lambda (line) #t))

;; Fired for each command you enter (before aliases/triggers).
(mud/on-input (lambda (cmd)
  ;; (mud/pane-print "main" (to-string "you typed: " cmd))
  #t))

;; Fired when the server sends an MSDP message. data is a hash.
(mud/on-msdp (lambda (data)
  ;; (mud/pane-print "main" (to-string "msdp " data))
  #t))
"##;

const DEFAULT_ON_GMCP: &str = r##";; Fired when the server sends a GMCP message. data is a hash.
(mud/on-gmcp (lambda (package data)
  ;; (mud/pane-print "main" (to-string "gmcp " package " " data))
  #t))
"##;

fn game_templates() -> Vec<Profile> {
  const DEFAULT_INPUT: &str = "(lambda (cmd) #t)";
  const DEFAULT_MSDP: &str = "(lambda (data) #t)";

  vec![
  GameTemplate::new("Achaea", ConnectionMode::Tcp, "achaea.com", 23, false)
    .theme("theme/onenord")
    .map_panes()
    .gauges(&[
      gauge("health", "red", "hp", "maxhp"),
      gauge("mana", "blue", "mp", "maxmp"),
      gauge("endurance", "green", "ep", "maxep")
    ])
    .connect()
    .on_line(
      r##"(lambda (line)
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
      (else #t))))"##
    )
    .on_input(DEFAULT_INPUT)
    .gmcp("Char.Vitals", &[
      gauge("health", "red", "hp", "maxhp"),
      gauge("mana", "blue", "mp", "maxmp"),
      gauge("endurance", "green", "ep", "maxep")
    ])
    .on_msdp(DEFAULT_MSDP)
    .concat(
      ";; Route map-like lines (box-drawing borders) to the map pane\n\
       (define in-map #f)\n\
       \n\
       ;; Aliases\n\
       (mud/alias \"^gg$\" (lambda ()\n  (mud/send \"get gold from corpse\")))\n\
       \n\
       (mud/alias \"^aa (.+)$\" (lambda (target)\n  (mud/send (to-string \"attack \" target))))\n"
    )
    .build(),
  GameTemplate::new("Aardwolf", ConnectionMode::Tcp, "aardmud.org", 23, false)
    .theme("theme/onenord")
    .gauges(&[
      gauge("health", "red", "hp", "maxhp"),
      gauge("mana", "blue", "mana", "maxmana"),
      gauge("moves", "yellow", "moves", "maxmoves")
    ])
    .connect()
    .default_hooks()
    .gmcp("char.vitals", &[
      gauge("health", "red", "hp", "maxhp"),
      gauge("mana", "blue", "mana", "maxmana"),
      gauge("moves", "yellow", "moves", "maxmoves")
    ])
    .concat(
      ";; Aliases\n\
       (mud/alias \"^sc$\" (lambda ()\n  (mud/send \"score\")))\n"
    )
    .build(),
  GameTemplate::new("BatMUD", ConnectionMode::Tcp, "batmud.bat.org", 23, false)
    .theme("theme/onenord")
    .gauges(&[
      gauge("health", "red", "hp", "maxhp"),
      gauge("sp", "blue", "sp", "maxsp"),
      gauge("ep", "green", "ep", "maxep")
    ])
    .connect()
    .default_hooks()
    .gmcp("Char.Vitals", &[
      gauge("health", "red", "hp", "maxhp"),
      gauge("sp", "blue", "sp", "maxsp"),
      gauge("ep", "green", "ep", "maxep")
    ])
    .build(),
  GameTemplate::new("Discworld", ConnectionMode::Tcp, "discworld.atuin.net", 4242, false)
    .theme("theme/onenord")
    .map_panes()
    .gauges(&[
      gauge("hp", "red", "hp", "maxhp"),
      gauge("gp", "blue", "gp", "maxgp"),
      gauge("xp", "yellow", "", "")
    ])
    .connect()
    .on_line(
      r##"(lambda (line)
  (let ((text (mud/strip-ansi line)))
    (cond
      ((mud/regexp-match? "^\\+[-]+\\+$" text)
       (set! in-map (not in-map))
       (mud/pane-print "map" line)
       #f)
      (in-map
       (mud/pane-print "map" line)
       #f)
      (else #t))))"##
    )
    .on_input(DEFAULT_INPUT)
    .gmcp("char.vitals", &[
      gauge("hp", "red", "hp", "maxhp"),
      gauge("gp", "blue", "gp", "maxgp")
    ])
    .on_msdp(DEFAULT_MSDP)
    .concat(
      ";; Route map blocks (delimited by +---+) to the map pane\n\
       (define in-map #f)\n\
       \n\
       ;; Aliases\n\
       (mud/alias \"^l$\" (lambda ()\n  (mud/send \"look\")))\n"
    )
    .build(),
  GameTemplate::new("GemStone IV", ConnectionMode::Tcp, "gemstone.net", 7777, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("DragonRealms", ConnectionMode::Tcp, "prime.dr.game.play.net", 4901, false)
    .theme("theme/onenord")
    .connect()
    .on_line(
      "(lambda (line)\n  (when (not launch-key-note-shown)\n    (set! launch-key-note-shown #t)\n    (mud/pane-print \"main\" \"[DragonRealms may need a launch key from the official Simutronics launcher.]\"))\n  #t)"
    )
    .on_input(DEFAULT_INPUT)
    .default_on_gmcp()
    .on_msdp(DEFAULT_MSDP)
    .concat("(define launch-key-note-shown #f)\n")
    .build(),
  GameTemplate::new("NukeFire", ConnectionMode::Tcp, "tdome.nukefire.org", 4000, false)
    .apply_nukefire_config()
    .build(),
  GameTemplate::new(
    "NukeFire WebSocket (experimental)",
    ConnectionMode::WebSocket,
    "tintin.nukefire.org",
    443,
    false
  )
  .websocket("wss://tintin.nukefire.org/ws", Some("tty".into()))
  .apply_nukefire_config()
  .concat(";; Uses the same endpoint and tty WebSocket protocol as the browser client.\n")
  .build(),
  GameTemplate::new("Threshold RPG", ConnectionMode::Tcp, "thresholdrpg.com", 3333, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("AwakeMUD CE", ConnectionMode::Tcp, "play.awakemud.com", 4000, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("Realms of Despair", ConnectionMode::Tcp, "realmsofdespair.com", 4000, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("Legends of the Jedi", ConnectionMode::Tcp, "legendsofthejedi.com", 5656, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("Miriani", ConnectionMode::Tcp, "toastsoft.net", 1234, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("Alter Aeon", ConnectionMode::Tcp, "alteraeon.com", 3000, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("Genesis", ConnectionMode::Tcp, "mud.genesismud.org", 3011, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("Genesis WebSocket (experimental)", ConnectionMode::WebSocket, "mud.genesismud.org", 3011, false)
    .theme("theme/onenord")
    .websocket("wss://www.genesismud.org/websocket", None)
    .connect()
    .default_hooks()
    .default_on_gmcp()
    .concat(";; Genesis documents this WebSocket endpoint for its official web client.\n")
    .build(),
  GameTemplate::new("The Eternal City", ConnectionMode::Tcp, "game.eternalcitygame.com", 6730, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("Materia Magica", ConnectionMode::Tcp, "materiamagica.com", 23, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("Avatar MUD", ConnectionMode::Tcp, "avatar.outland.org", 3000, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("Star Trek: Phoenix Rising", ConnectionMode::Tcp, "game.phxrising.org", 1701, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("CLOK", ConnectionMode::Tcp, "clok.contrarium.net", 4000, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("CoffeeMud", ConnectionMode::Tcp, "coffeemud.net", 23, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("MUME", ConnectionMode::Tcp, "mume.org", 4242, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("MUME WebSocket (experimental)", ConnectionMode::WebSocket, "mume.org", 4242, false)
    .theme("theme/onenord")
    .websocket("wss://mume.org/ws-play/", None)
    .connect()
    .default_hooks()
    .default_on_gmcp()
    .concat(";; Experimental: MUME documents an official WebSocket proxy at this path.\n")
    .build(),
  GameTemplate::new("Icesus MUD", ConnectionMode::Tcp, "icesus.org", 4000, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("Dune", ConnectionMode::Tcp, "dunemud.net", 6789, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("Merentha", ConnectionMode::Tcp, "merentha.com", 10000, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("Lost Souls", ConnectionMode::Tcp, "lostsouls.org", 23, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("RetroMUD", ConnectionMode::Tcp, "retromud.org", 3000, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("Mirkwood", ConnectionMode::Tcp, "mirkwoodmud.org", 4000, false)
    .theme("theme/onenord")
    .connect().default_hooks().default_on_gmcp().build(),
  GameTemplate::new("Enrym TCP", ConnectionMode::Tcp, "play.enrym.com", 4001, true)
    .theme("theme/onenord")
    .connect()
    .default_hooks()
    .on_gmcp(
      "(lambda (package data)\n  (mud/pane-print \"main\" (to-string \"[GMCP \" package \"]\")))"
    )
    .concat(";; Log GMCP messages\n")
    .build(),
  GameTemplate::new("Enrym WebSocket", ConnectionMode::WebSocket, "play.enrym.com", 4001, true)
    .theme("theme/onenord")
    .websocket("wss://play.enrym.com", None)
    .connect()
    .default_hooks()
    .on_gmcp(
      "(lambda (package data)\n  (mud/pane-print \"main\" (to-string \"[GMCP \" package \"]\")))"
    )
    .concat(";; Log GMCP messages\n")
    .build(),
  GameTemplate::new("Generic", ConnectionMode::Tcp, "localhost", 4000, false)
    .theme("theme/onenord")
    .connect()
    .default_hooks()
    .on_gmcp(
      "(lambda (package data)\n  (mud/pane-print \"main\" (to-string \"[GMCP \" package \"]\")))"
    )
    .concat(";; Log GMCP messages\n")
    .build()
  ]
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
    game_templates()
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
          name: fallback_name,
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
    self.name = new_name.to_string();
    self.path = Some(new_dir.join("init.scm"));
    Ok(())
  }
}

#[derive(Default)]
struct ProfileMetadata {
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
    if let Some(val) =
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

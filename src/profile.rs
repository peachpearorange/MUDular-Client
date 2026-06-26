use std::fmt::Write;
use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
use directories::ProjectDirs;

#[derive(Clone, Debug)]
pub struct Profile {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub tls: bool,
    pub script_code: String,
    pub path: Option<PathBuf>,
    pub is_preset: bool,
}

struct GaugeTemplate {
    name: &'static str,
    color: &'static str,
    gmcp_cur: &'static str,
    gmcp_max: &'static str,
}

struct GameTemplate {
    name: &'static str,
    host: &'static str,
    port: u16,
    tls: bool,
    has_map: bool,
    gauges: &'static [GaugeTemplate],
    gmcp_package: &'static str,
    extra_scheme: &'static str,
}

const GAME_TEMPLATES: &[GameTemplate] = &[
    GameTemplate {
        name: "Achaea",
        host: "achaea.com",
        port: 23,
        tls: false,
        has_map: true,
        gauges: &[
            GaugeTemplate { name: "health", color: "red", gmcp_cur: "hp", gmcp_max: "maxhp" },
            GaugeTemplate { name: "mana", color: "blue", gmcp_cur: "mp", gmcp_max: "maxmp" },
            GaugeTemplate { name: "endurance", color: "green", gmcp_cur: "ep", gmcp_max: "maxep" },
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
"#,
    },
    GameTemplate {
        name: "Aardwolf",
        host: "aardmud.org",
        port: 23,
        tls: false,
        has_map: false,
        gauges: &[
            GaugeTemplate { name: "health", color: "red", gmcp_cur: "hp", gmcp_max: "maxhp" },
            GaugeTemplate { name: "mana", color: "blue", gmcp_cur: "mana", gmcp_max: "maxmana" },
            GaugeTemplate { name: "moves", color: "yellow", gmcp_cur: "moves", gmcp_max: "maxmoves" },
        ],
        gmcp_package: "char.vitals",
        extra_scheme: r#";; Aliases
(alias "^sc$" (lambda ()
  (send "score")))
"#,
    },
    GameTemplate {
        name: "BatMUD",
        host: "batmud.bat.org",
        port: 23,
        tls: false,
        has_map: false,
        gauges: &[
            GaugeTemplate { name: "health", color: "red", gmcp_cur: "hp", gmcp_max: "maxhp" },
            GaugeTemplate { name: "sp", color: "blue", gmcp_cur: "sp", gmcp_max: "maxsp" },
            GaugeTemplate { name: "ep", color: "green", gmcp_cur: "ep", gmcp_max: "maxep" },
        ],
        gmcp_package: "Char.Vitals",
        extra_scheme: "",
    },
    GameTemplate {
        name: "Discworld",
        host: "discworld.atuin.net",
        port: 4242,
        tls: false,
        has_map: true,
        gauges: &[
            GaugeTemplate { name: "hp", color: "red", gmcp_cur: "hp", gmcp_max: "maxhp" },
            GaugeTemplate { name: "gp", color: "blue", gmcp_cur: "gp", gmcp_max: "maxgp" },
            GaugeTemplate { name: "xp", color: "yellow", gmcp_cur: "", gmcp_max: "" },
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
"#,
    },
    GameTemplate {
        name: "Enrym",
        host: "play.enrym.com",
        port: 4001,
        tls: true,
        has_map: false,
        gauges: &[],
        gmcp_package: "",
        extra_scheme: r#";; Log GMCP messages
(define (on-gmcp package data)
  (pane-print "main" (to-string "[GMCP " package "]")))
"#,
    },
    GameTemplate {
        name: "Generic",
        host: "localhost",
        port: 4000,
        tls: false,
        has_map: false,
        gauges: &[],
        gmcp_package: "",
        extra_scheme: r#";; Log GMCP messages
(define (on-gmcp package data)
  (pane-print "main" (to-string "[GMCP " package "]")))
"#,
    },
];

fn generate_scheme(t: &GameTemplate) -> String {
    let mut s = String::new();

    let _ = writeln!(s, "(define name \"{}\")", t.name);
    let _ = writeln!(s, "(define host \"{}\")", t.host);
    let _ = writeln!(s, "(define port {})", t.port);
    let _ = writeln!(s, "(define tls {})", if t.tls { "#t" } else { "#f" });
    let _ = writeln!(s);
    let _ = writeln!(s, ";; You can use any of 550+ built-in themes from https://iterm2colorschemes.com");
    let _ = writeln!(s, "(load-theme \"Onenord\")");
    let _ = writeln!(s, "(option \"scroll_lines\" 3)");
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
                let _ = writeln!(s, "    (when (and (hash-contains? data \"{}\") (hash-contains? data \"{}\"))", g.gmcp_cur, g.gmcp_max);
                let _ = writeln!(
                    s, "      (gauge \"{}\" (hash 'current (hash-ref data \"{}\") 'max (hash-ref data \"{}\") 'color \"{}\")))",
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
        { ProjectDirs::from("com", "mudular", "mudular-client")
            .map(|dirs| dirs.config_dir().join("profiles")) }
        #[cfg(target_arch = "wasm32")]
        None
    }

    pub fn load_user() -> Vec<Profile> {
        #[cfg(not(target_arch = "wasm32"))]
        { Self::load_user_profiles() }
        #[cfg(target_arch = "wasm32")]
        Vec::new()
    }

    pub fn templates() -> Vec<Profile> {
        let mut templates: Vec<Profile> = GAME_TEMPLATES.iter().map(|t| Profile {
            name: t.name.into(),
            host: t.host.into(),
            port: t.port,
            tls: t.tls,
            script_code: generate_scheme(t),
            path: None,
            is_preset: true,
        }).collect();
        templates.push(Profile {
            name: "NukeFire".into(),
            host: "tdome.nukefire.org".into(),
            port: 4000,
            tls: false,
            script_code: include_str!("../profiles/nukefire/init.scm").into(),
            path: None,
            is_preset: true,
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
                let (path, code) = std::fs::read_to_string(&scm_path).ok()
                    .map(|c| (scm_path, c))
                    .or_else(|| std::fs::read_to_string(&lua_path).ok().map(|c| (lua_path, c)))?;
                let name = e.file_name().to_string_lossy().to_string();
                Some(Profile {
                    name: name.clone(),
                    host: extract_scheme_string(&code, "host").unwrap_or_else(|| "localhost".into()),
                    port: extract_scheme_number(&code, "port").unwrap_or(4000.0) as u16,
                    tls: extract_scheme_bool(&code, "tls"),
                    script_code: code,
                    path: Some(path),
                    is_preset: false,
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
        let old_name_line = format!("(define name \"{}\")", self.name);
        let new_name_line = format!("(define name \"{}\")", new_name);
        self.script_code = self.script_code.replacen(&old_name_line, &new_name_line, 1);
        self.name = new_name.to_string();
        let scm_path = new_dir.join("init.scm");
        std::fs::write(&scm_path, &self.script_code).map_err(|e| e.to_string())?;
        self.path = Some(scm_path);
        Ok(())
    }
}

fn extract_scheme_string(code: &str, var: &str) -> Option<String> {
    let pattern = format!("(define {var} \"");
    let start = code.find(&pattern)? + pattern.len();
    let end = code[start..].find('"')? + start;
    Some(code[start..end].to_string())
}

fn extract_scheme_bool(code: &str, var: &str) -> bool {
    let pattern = format!("(define {var} ");
    code.find(&pattern)
        .map(|i| code[i + pattern.len()..].starts_with("#t"))
        .unwrap_or(false)
}

fn extract_scheme_number(code: &str, var: &str) -> Option<f64> {
    let pattern = format!("(define {var} ");
    let start = code.find(&pattern)? + pattern.len();
    let end = code[start..].find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(code[start..].len()) + start;
    code[start..end].parse().ok()
}

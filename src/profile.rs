use std::fmt::Write;
use std::path::PathBuf;

use directories::ProjectDirs;

#[derive(Clone, Debug)]
pub struct Profile {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub tls: bool,
    pub lua_code: String,
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
    extra_lua: &'static str,
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
        extra_lua: r#"local in_map = false

function on_line(line)
    if string.find(line, "^%s*%-%-%-%-%-%-%-%-%+") or string.find(line, "^%s*|.*|%s*$") then
        in_map = true
        map:print(line)
        return false
    end
    if in_map then
        if line == "" or not string.find(line, "[|%-+]") then
            in_map = false
        else
            map:print(line)
            return false
        end
    end
    return true
end

mud.alias("^gg$", function()
    mud.send("get gold from corpse")
end)

mud.alias("^aa (.+)$", function(target)
    mud.send("attack " .. target)
end)
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
        extra_lua: r#"mud.alias("^sc$", function()
    mud.send("score")
end)
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
        extra_lua: "",
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
        extra_lua: r#"local in_map = false

function on_line(line)
    if string.find(line, "^%+[-]+%+$") then
        in_map = not in_map
        map:print(line)
        return false
    end
    if in_map then
        map:print(line)
        return false
    end
    return true
end

mud.alias("^l$", function()
    mud.send("look")
end)
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
        extra_lua: r#"function on_gmcp(package, data)
    main:print("[GMCP " .. package .. "]")
end
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
        extra_lua: r#"function on_gmcp(package, data)
    main:print("[GMCP " .. package .. "]")
end
"#,
    },
];

fn generate_lua(t: &GameTemplate) -> String {
    let mut s = String::new();

    let _ = writeln!(s, "name = \"{}\"", t.name);
    let _ = writeln!(s, "host = \"{}\"", t.host);
    let _ = writeln!(s, "port = {}", t.port);
    let _ = writeln!(s, "tls = {}", t.tls);
    let _ = writeln!(s);
    let _ = writeln!(s, "mud.load_theme(\"Onenord\")");
    let _ = writeln!(s, "mud.option(\"scroll_lines\", 3)");
    let _ = writeln!(s);
    let _ = writeln!(s, "mud.keymap(\"PageUp\", \"scroll_up 20\")");
    let _ = writeln!(s, "mud.keymap(\"PageDown\", \"scroll_down 20\")");
    let _ = writeln!(s);
    let _ = writeln!(s, "local main = mud.pane(\"main\")");

    if t.has_map {
        let _ = writeln!(s, "local map = mud.pane(\"map\")");
        let _ = writeln!(s);
        let _ = writeln!(s, "mud.layout(\"horizontal\", {{");
        let _ = writeln!(s, "    {{ pane = \"main\", weight = 3 }},");
        let _ = writeln!(s, "    {{ pane = \"map\", weight = 1 }},");
        let _ = writeln!(s, "}})");
    }
    let _ = writeln!(s);

    for g in t.gauges {
        let _ = writeln!(
            s, "mud.gauge(\"{}\", {{ color = \"{}\" }})",
            g.name, g.color,
        );
    }
    let _ = writeln!(s);

    let _ = writeln!(s, "function on_connect()");
    let _ = writeln!(s, "    main:print(\"[Connected to {}]\")", t.name);
    let _ = writeln!(s, "end");
    let _ = writeln!(s);
    let _ = writeln!(s, "function on_disconnect()");
    let _ = writeln!(s, "    main:print(\"[Disconnected from {}]\")", t.name);
    let _ = writeln!(s, "end");

    if !t.extra_lua.contains("function on_line") {
        let _ = writeln!(s);
        let _ = writeln!(s, "function on_line(line)");
        let _ = writeln!(s, "    return true");
        let _ = writeln!(s, "end");
    }

    if !t.extra_lua.contains("function on_gmcp") && !t.gmcp_package.is_empty() {
        let _ = writeln!(s);
        let _ = writeln!(s, "function on_gmcp(package, data)");
        let _ = writeln!(s, "    if package == \"{}\" then", t.gmcp_package);
        for g in t.gauges {
            if !g.gmcp_cur.is_empty() {
                let _ = writeln!(s, "        if data.{} and data.{} then", g.gmcp_cur, g.gmcp_max);
                let _ = writeln!(
                    s, "            mud.gauge(\"{}\", {{ current = tonumber(data.{}), max = tonumber(data.{}), color = \"{}\" }})",
                    g.name, g.gmcp_cur, g.gmcp_max, g.color,
                );
                let _ = writeln!(s, "        end");
            }
        }
        let _ = writeln!(s, "    end");
        let _ = writeln!(s, "end");
    }

    if !t.extra_lua.is_empty() {
        let _ = writeln!(s);
        s.push_str(t.extra_lua);
    }

    s
}

impl Profile {
    pub fn profiles_dir() -> Option<PathBuf> {
        ProjectDirs::from("com", "mudular", "MUDular Client")
            .map(|dirs| dirs.config_dir().join("profiles"))
    }

    pub fn load_user() -> Vec<Profile> {
        Self::load_user_profiles()
    }

    pub fn templates() -> Vec<Profile> {
        let mut templates: Vec<Profile> = GAME_TEMPLATES.iter().map(|t| Profile {
            name: t.name.into(),
            host: t.host.into(),
            port: t.port,
            tls: t.tls,
            lua_code: generate_lua(t),
            path: None,
            is_preset: true,
        }).collect();
        templates.push(Profile {
            name: "NukeFire".into(),
            host: "tdome.nukefire.org".into(),
            port: 4000,
            tls: false,
            lua_code: include_str!("../profiles/nukefire/init.lua").into(),
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
                let lua_path = e.path().join("init.lua");
                let code = std::fs::read_to_string(&lua_path).ok()?;
                let name = e.file_name().to_string_lossy().to_string();
                Some(Profile {
                    name: name.clone(),
                    host: extract_lua_string(&code, "host").unwrap_or_else(|| "localhost".into()),
                    port: extract_lua_number(&code, "port").unwrap_or(4000.0) as u16,
                    tls: extract_lua_bool(&code, "tls"),
                    lua_code: code,
                    path: Some(lua_path),
                    is_preset: false,
                })
            })
            .collect()
    }

    pub fn save(&mut self) -> Result<(), String> {
        let dir = Self::profiles_dir().ok_or("Could not determine config directory")?;
        let profile_dir = dir.join(&self.name);
        std::fs::create_dir_all(&profile_dir).map_err(|e| e.to_string())?;
        let lua_path = profile_dir.join("init.lua");
        std::fs::write(&lua_path, &self.lua_code).map_err(|e| e.to_string())?;
        self.path = Some(lua_path);
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
        let old_name_line = format!("name = \"{}\"", self.name);
        let new_name_line = format!("name = \"{}\"", new_name);
        self.lua_code = self.lua_code.replacen(&old_name_line, &new_name_line, 1);
        self.name = new_name.to_string();
        let lua_path = new_dir.join("init.lua");
        std::fs::write(&lua_path, &self.lua_code).map_err(|e| e.to_string())?;
        self.path = Some(lua_path);
        Ok(())
    }
}

fn extract_lua_string(code: &str, var: &str) -> Option<String> {
    let pattern = format!("{var} = \"");
    let start = code.find(&pattern)? + pattern.len();
    let end = code[start..].find('"')? + start;
    Some(code[start..end].to_string())
}

fn extract_lua_bool(code: &str, var: &str) -> bool {
    let pattern = format!("{var} = ");
    code.find(&pattern)
        .map(|i| code[i + pattern.len()..].starts_with("true"))
        .unwrap_or(false)
}

fn extract_lua_number(code: &str, var: &str) -> Option<f64> {
    let pattern = format!("{var} = ");
    let start = code.find(&pattern)? + pattern.len();
    let end = code[start..].find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(code[start..].len()) + start;
    code[start..end].parse().ok()
}

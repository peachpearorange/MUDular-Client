mod ansi;
mod app;
mod buffer;
mod connection;
mod probe;
mod profile;
mod protocol;
mod scripting;
mod telnet;
mod themes;
mod ui;

struct DualWriter {
    stderr: std::io::Stderr,
    file: std::sync::Mutex<std::fs::File>,
}

impl std::io::Write for DualWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let _ = self.stderr.write(buf);
        let _ = self.file.lock().unwrap().write(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        let _ = self.stderr.flush();
        let _ = self.file.lock().unwrap().flush();
        Ok(())
    }
}

fn main() -> eframe::Result<()> {
    let log_path = "/tmp/mudular.log";
    let log_file = std::fs::File::create(log_path).expect("Failed to create log file");
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn,mudular=info")
    )
    .target(env_logger::Target::Pipe(Box::new(DualWriter {
        stderr: std::io::stderr(),
        file: std::sync::Mutex::new(log_file),
    })))
    .init();
    eprintln!("Logging to {log_path}");
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([640.0, 480.0]),
        ..Default::default()
    };

    eframe::run_native(
        "MUDular Client",
        options,
        Box::new(|cc| Ok(Box::new(app::MudApp::new(cc)))),
    )
}

#[cfg(test)]
mod tests {
    use crate::scripting::ScriptEngine;

    #[test]
    fn test_nukefire_profile_loads() {
        let code = include_str!("../profiles/nukefire/init.lua");
        let mut engine = ScriptEngine::new().expect("engine creation failed");
        engine.load_script(code).expect("nukefire script failed to load");
        
        let st = engine.state.lock().unwrap();
        assert!(st.panes.contains_key("main"));
        assert!(st.panes.contains_key("map"));
        assert!(st.gauges.len() >= 3);
    }
    
    #[test]
    fn test_nukefire_on_line() {
        let code = include_str!("../profiles/nukefire/init.lua");
        let mut engine = ScriptEngine::new().expect("engine creation failed");
        engine.load_script(code).expect("script load failed");
        
        // Normal text should pass through
        assert!(engine.handle_line("Hello world"));
        
        // Prompt echo should be suppressed
        assert!(!engine.handle_line("> n"));
        
        // Map grid should be captured
        assert!(!engine.handle_line("  |  @  |  "));
    }
    
    #[test]
    fn test_generated_templates_load() {
        let templates = crate::profile::Profile::templates();
        for template in &templates {
            let mut engine = ScriptEngine::new().expect("engine creation failed");
            engine.load_script(&template.lua_code)
                .unwrap_or_else(|e| panic!("template '{}' failed to load: {e}", template.name));
            let st = engine.state.lock().unwrap();
            assert!(st.panes.contains_key("main"), "template '{}' missing main pane", template.name);
            assert!(st.ansi_palette.is_some(), "template '{}' missing palette (load_theme failed)", template.name);
        }
    }

    #[test]
    fn test_nukefire_keymaps() {
        let code = include_str!("../profiles/nukefire/init.lua");
        let mut engine = ScriptEngine::new().expect("engine creation failed");
        engine.load_script(code).expect("script load failed");

        let st = engine.state.lock().unwrap();
        assert!(st.keymaps.len() >= 6);
        assert!(st.keep_input);
        let w_map = st.keymaps.iter().find(|km| km.combo.key == "w").unwrap();
        assert!(w_map.combo.alt);
        assert_eq!(w_map.command, "n");
    }
}

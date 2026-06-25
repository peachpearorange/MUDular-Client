use eframe::egui;

pub struct InputLine {
    pub text: String,
    pub history: Vec<String>,
    pub history_pos: Option<usize>,
    pub submitted: Option<String>,
    pub keymap_matched: bool,
    select_all_next_frame: bool,
}

impl InputLine {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            history: Vec::new(),
            history_pos: None,
            submitted: None,
            keymap_matched: false,
            select_all_next_frame: false,
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui, connected: bool, keep_input: bool, suppress_focus: bool) {
        self.submitted = None;

        ui.horizontal(|ui| {
            let font_id = ui.style().text_styles.get(&egui::TextStyle::Monospace)
                .cloned()
                .unwrap_or_else(|| egui::FontId::monospace(13.0));

            let text_before = self.text.clone();
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.text)
                    .font(font_id)
                    .frame(egui::Frame::NONE)
                    .margin(egui::Margin::ZERO)
                    .desired_width(ui.available_width())
                    .interactive(connected),
            );

            if self.keymap_matched && self.text != text_before {
                self.text = text_before;
            }
            self.keymap_matched = false;

            if self.select_all_next_frame {
                self.select_all_next_frame = false;
                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), response.id) {
                    let len = self.text.chars().count();
                    state.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                        egui::text::CCursor::new(0),
                        egui::text::CCursor::new(len),
                    )));
                    state.store(ui.ctx(), response.id);
                }
            }

            if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && connected {
                self.submit(keep_input);
                response.request_focus();
            }

            if connected && !response.has_focus() && !suppress_focus {
                let skip = ui.input(|i| {
                    (i.modifiers.command && i.key_pressed(egui::Key::C))
                    || i.modifiers.alt
                });

                if !skip {
                    ui.input(|i| {
                        for e in &i.events {
                            if let egui::Event::Text(t) = e {
                                self.text.push_str(t);
                            }
                        }
                    });

                    if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.submit(keep_input);
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                        self.history_up();
                    }
                    if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                        self.history_down();
                    }

                    response.request_focus();
                }
            }

            if response.has_focus() {
                if ui.input(|i| i.key_pressed(egui::Key::ArrowUp)) {
                    self.history_up();
                }
                if ui.input(|i| i.key_pressed(egui::Key::ArrowDown)) {
                    self.history_down();
                }
            }

        });
    }

    pub fn take_submitted(&mut self) -> Option<String> {
        self.submitted.take()
    }

    fn submit(&mut self, keep_input: bool) {
        let cmd = self.text.trim().to_string();
        if !cmd.is_empty() {
            self.history.push(cmd.clone());
        }
        self.submitted = Some(cmd);
        if keep_input {
            self.select_all_next_frame = true;
        } else {
            self.text.clear();
        }
        self.history_pos = None;
    }

    fn history_up(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let pos = self.history_pos
            .map(|p| p.saturating_sub(1))
            .unwrap_or(self.history.len() - 1);
        self.history_pos = Some(pos);
        self.text = self.history[pos].clone();
    }

    fn history_down(&mut self) {
        let Some(pos) = self.history_pos else { return };
        if pos + 1 >= self.history.len() {
            self.history_pos = None;
            self.text.clear();
        } else {
            self.history_pos = Some(pos + 1);
            self.text = self.history[pos + 1].clone();
        }
    }
}

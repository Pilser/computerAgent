//! `eframe::App`: owns engine + state + shell + pump.
//!
//! `ui()` = drain pump → fold events into state → `shell.run()`.
//! Repaint is input-driven except while `sending`, when streamed chunks
//! force it.

use std::sync::Arc;

use computer_agent_engine::Engine;

use crate::shell::Shell;
use crate::state::ChatState;
use crate::stream::{Pump, PumpEvent};

/// The egui application: one shared engine, one state tree.
pub struct ComputerAgentApp {
    engine: Arc<Engine>,
    state: ChatState,
    shell: Shell,
    pump: Pump,
    loaders_installed: bool,
}

impl ComputerAgentApp {
    /// Build against the production engine + a fresh pump thread.
    pub fn new() -> Self {
        let engine = Arc::new(Engine::new());
        let mut state = ChatState::new(Engine::chat_models(), &engine.current_model());
        state.refresh_from_engine(&engine);
        let pump = Pump::new(Arc::clone(&engine));
        Self {
            engine,
            state,
            shell: Shell::new(),
            pump,
            loaders_installed: false,
        }
    }

    /// Active thread id for run dispatch (`None` = ad-hoc session).
    fn active_thread(&self) -> Option<String> {
        self.state.threads.active_id.clone()
    }
}

impl Default for ComputerAgentApp {
    fn default() -> Self {
        Self::new()
    }
}

impl eframe::App for ComputerAgentApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.loaders_installed {
            egui_extras::install_image_loaders(ui.ctx());
            self.loaders_installed = true;
        }
        if let Some(prompt) = self.state.chat.take_send_request() {
            self.pump.send(prompt, self.active_thread());
        }
        for event in self.pump.drain() {
            match event {
                PumpEvent::Chunk(chunk) => {
                    self.state.chat.append_chunk(&chunk);
                }
                PumpEvent::Done(response) => {
                    self.state.chat.finish_assistant(&response);
                    self.state
                        .status
                        .set_ok(&format!("{} messages", self.state.chat.messages().len()));
                    self.state.refresh_from_engine(&self.engine);
                    self.state
                        .refresh_usage(&self.engine, self.active_thread().as_deref());
                }
                PumpEvent::Failed(err) => {
                    self.state.chat.sending = false;
                    self.state.status.set_err(&err);
                }
            }
        }
        if self.state.chat.sending {
            ui.ctx().request_repaint();
        }
        self.shell.run(ui, &mut self.state, &self.engine);
    }
}

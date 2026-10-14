//! desktop-egui: native window running the egui frontend.
//!
//! Builds one [`ComputerAgentApp`] and hands it to `eframe::run_native`.
//! The Slint desktop binary keeps shipping until screenshot parity.

use anyhow::Context;
use computer_agent_frontend_egui::ComputerAgentApp;

fn main() -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "computerAgent",
        options,
        Box::new(|_cc| Ok(Box::new(ComputerAgentApp::new()) as Box<dyn eframe::App>)),
    )
    .context("run egui native app")
}

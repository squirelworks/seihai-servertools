//! One module per page. See the crate docs for how to add one.

pub mod check;
pub mod host;
pub mod lab;
pub mod setup;

use eframe::egui;

/// A page that is not built yet: say what it will do and where the plan is.
pub fn planned(ui: &mut egui::Ui, what: &str) {
    ui.label("Not built yet.");
    ui.label(what);
    ui.label("Order of the work: docs/NEXT.md.");
}

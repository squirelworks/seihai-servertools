//! The seihai-servertools window (egui, the toolkit of the Seihai client and
//! its level editor).
//!
//! One page per tool. Pages hold only UI state and call into `setup-core`
//! and `lat-core`; no rule or file format lives here. Adding a page:
//! write `src/pages/<name>.rs` with a `Page` struct and `fn show(&mut self,
//! ui)`, add it to [`Tab`] and to [`App::update`].

mod pages;

use eframe::egui;

/// The pages, in the order of the side bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Lab,
    Setup,
    Levels,
    Npcs,
    Ganis,
    Check,
}

impl Tab {
    pub const ALL: [Tab; 6] = [
        Tab::Lab,
        Tab::Setup,
        Tab::Levels,
        Tab::Npcs,
        Tab::Ganis,
        Tab::Check,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Tab::Lab => "Lab world",
            Tab::Setup => "Server setup",
            Tab::Levels => "Level editor (LAT)",
            Tab::Npcs => "NPC editor",
            Tab::Ganis => "Gani editor",
            Tab::Check => "World check",
        }
    }
}

/// The window's state.
pub struct App {
    pub tab: Tab,
    lab: pages::lab::Page,
    check: pages::check::Page,
}

impl Default for App {
    fn default() -> Self {
        App {
            tab: Tab::Lab,
            lab: pages::lab::Page::default(),
            check: pages::check::Page::default(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("tabs").show(ctx, |ui| {
            ui.heading("Seihai server tools");
            ui.separator();
            for tab in Tab::ALL {
                ui.selectable_value(&mut self.tab, tab, tab.title());
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(self.tab.title());
            ui.separator();
            match self.tab {
                Tab::Lab => self.lab.show(ui),
                Tab::Check => self.check.show(ui),
                Tab::Setup => pages::planned(
                    ui,
                    "Join a listserver, Host a listserver and Back to lab. Design: docs/design/go-public.md.",
                ),
                Tab::Levels => pages::planned(
                    ui,
                    "Open a level on a live server through the RC file browser, edit it, upload it with LAT rights. Plan: docs/design/lat-tools.md.",
                ),
                Tab::Npcs => pages::planned(
                    ui,
                    "Level NPC scripts, and database NPCs, classes and weapons through an NC login. Plan: docs/design/lat-tools.md.",
                ),
                Tab::Ganis => pages::planned(
                    ui,
                    "Frames, sprites and a preview, uploaded through the file link. Plan: docs/design/lat-tools.md.",
                ),
            }
        });
    }
}

/// Open the window and run until it is closed.
pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Seihai server tools")
            .with_inner_size([960.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Seihai server tools",
        options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}

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
    setup: pages::setup::Page,
    check: pages::check::Page,
}

impl Default for App {
    fn default() -> Self {
        App {
            tab: Tab::Lab,
            lab: pages::lab::Page::default(),
            setup: pages::setup::Page::default(),
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
                Tab::Setup => self.setup.show(ui),
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

/// What the window opens with.
#[derive(Debug, Clone, Default)]
pub struct StartOptions {
    /// A world folder to fill in (and open on the setup page).
    pub world: Option<std::path::PathBuf>,
    /// The page to show first.
    pub tab: Option<Tab>,
}

impl Tab {
    /// A page by a short name: lab, setup, levels, npcs, ganis, check.
    pub fn from_name(name: &str) -> Option<Tab> {
        Some(match name {
            "lab" => Tab::Lab,
            "setup" => Tab::Setup,
            "levels" => Tab::Levels,
            "npcs" => Tab::Npcs,
            "ganis" => Tab::Ganis,
            "check" => Tab::Check,
            _ => return None,
        })
    }
}

impl App {
    /// A window state with `opts` applied.
    pub fn with_options(opts: &StartOptions) -> Self {
        let mut app = App::default();
        if let Some(tab) = opts.tab {
            app.tab = tab;
        }
        if let Some(w) = &opts.world {
            let w = w.display().to_string();
            app.lab.set_world(&w);
            app.check.set_world(&w);
            app.setup.open_world(&w);
        }
        app
    }
}

/// Open the window and run until it is closed.
pub fn run(opts: StartOptions) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Seihai server tools")
            .with_inner_size([1000.0, 760.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Seihai server tools",
        options,
        Box::new(move |_cc| Ok(Box::new(App::with_options(&opts)))),
    )
}

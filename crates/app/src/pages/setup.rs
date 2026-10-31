//! Server setup page: the Join a listserver wizard, Back to lab, undo, and
//! Start and check. Every rule, check and write is in `setup_core`
//! (`folder::ServerFolder`, `checks`, `lights`, `net`); this page only holds
//! what the owner typed and shows results.

use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use eframe::egui::{self, Color32, RichText};
use setup_core::checks::{Finding, Severity};
use setup_core::folder::{Preview, ServerFolder};
use setup_core::lab::exe_name;
use setup_core::lights::{Light, Lights};
use setup_core::net::{test_connection, Reach};
use setup_core::profile::{Profile, PublicAnswers};
use setup_core::run::{start, Group, ProcessSpec};

/// The owner's answers as typed (strings, so half-typed values are kept).
struct Form {
    account: String,
    name: String,
    description: String,
    url: String,
    interface: String,
    server_ip: String,
    list_host: String,
    list_port: String,
    upnp: bool,
}

impl Default for Form {
    fn default() -> Self {
        Form {
            account: String::new(),
            name: String::new(),
            description: String::new(),
            url: String::new(),
            interface: "AUTO".into(),
            server_ip: "AUTO".into(),
            // Empty on purpose: no built-in listserver (AGENTS.md).
            list_host: String::new(),
            list_port: "14900".into(),
            upnp: false,
        }
    }
}

impl Form {
    fn from_folder(f: &ServerFolder) -> Self {
        let get = |k: &str| f.options.get(k).unwrap_or("").to_string();
        Form {
            name: get("name"),
            description: get("description"),
            url: get("url"),
            ..Form::default()
        }
    }

    fn answers(&self) -> Result<PublicAnswers, String> {
        Ok(PublicAnswers {
            interface: self.interface.trim().to_string(),
            server_ip: self.server_ip.trim().to_string(),
            list_host: self.list_host.trim().to_string(),
            list_port: self
                .list_port
                .trim()
                .parse()
                .map_err(|_| format!("listserver port {:?} is not a number", self.list_port))?,
            upnp: self.upnp,
            name: self.name.trim().to_string(),
            description: self.description.trim().to_string(),
            url: self.url.trim().to_string(),
            staff_account: self.account.trim().to_string(),
        })
    }
}

/// A started server and its console, for the lights.
struct Started {
    group: Group,
    console: std::path::PathBuf,
    public: bool,
}

pub struct Page {
    world: String,
    server: String,
    folder: Option<ServerFolder>,
    form: Form,
    preview: Option<Preview>,
    message: String,
    test: Option<Receiver<Reach>>,
    test_result: String,
    started: Option<Started>,
}

impl Default for Page {
    fn default() -> Self {
        Page {
            world: String::new(),
            server: "default".into(),
            folder: None,
            form: Form::default(),
            preview: None,
            message: String::new(),
            test: None,
            test_result: String::new(),
            started: None,
        }
    }
}

fn finding_line(ui: &mut egui::Ui, f: &Finding) {
    let (tag, color) = match f.severity {
        Severity::Block => ("BLOCKED", Color32::from_rgb(220, 60, 60)),
        Severity::Warn => ("warning", Color32::from_rgb(220, 160, 40)),
        Severity::Info => ("note", Color32::GRAY),
    };
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(tag).color(color).strong());
        if let Some(k) = &f.key {
            ui.monospace(k);
        }
        ui.label(&f.text);
    });
}

fn light_line(ui: &mut egui::Ui, title: &str, l: &Light) {
    let color = match l {
        Light::Ok(_) => Color32::from_rgb(60, 180, 90),
        Light::Warn(_) => Color32::from_rgb(220, 160, 40),
        Light::Bad(_) => Color32::from_rgb(220, 60, 60),
        Light::Waiting | Light::Unknown(_) => Color32::GRAY,
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new("●").color(color));
        ui.label(format!("{title}: {l}"));
    });
}

fn field(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::TextEdit::singleline(value).hint_text(hint));
    });
}

impl Page {
    /// Fill in the world folder and open its `default` server.
    pub fn open_world(&mut self, world: &str) {
        self.world = world.to_string();
        self.open();
    }

    fn open(&mut self) {
        self.preview = None;
        match ServerFolder::open(std::path::Path::new(self.world.trim()), self.server.trim()) {
            Ok(f) => {
                self.form = Form::from_folder(&f);
                self.message = String::new();
                self.folder = Some(f);
            }
            Err(e) => {
                self.folder = None;
                self.message = e;
            }
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("World folder:");
            ui.add(
                egui::TextEdit::singleline(&mut self.world)
                    .hint_text("folder with gs2emu and servers/"),
            );
            ui.label("Server:");
            ui.add(egui::TextEdit::singleline(&mut self.server).desired_width(100.0));
            if ui.button("Open").clicked() {
                self.open();
            }
        });
        if !self.message.is_empty() {
            ui.label(&self.message);
        }
        let Some(folder) = self.folder.as_ref() else {
            ui.label("Open a server folder to start.");
            return;
        };
        let profile = folder.profile();
        ui.label(format!(
            "{}: {}",
            folder.dir.display(),
            match profile {
                Profile::Lab => "Lab (loopback only)",
                _ => "Public",
            }
        ));
        let status = folder.status_findings();
        let options_path = folder.options_path.clone();

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.separator();
            ui.heading("1. Safety check");
            if status.is_empty() {
                ui.label("Nothing to report.");
            }
            for f in &status {
                finding_line(ui, f);
            }
            field(ui, "Your account (goes in staff):", &mut self.form.account, "your account name");

            ui.heading("2. Server details");
            field(ui, "Name:", &mut self.form.name, "shown in the server list");
            field(ui, "Description:", &mut self.form.description, "");
            field(ui, "Web page (optional):", &mut self.form.url, "");

            ui.heading("3. Network");
            field(ui, "Listen on:", &mut self.form.interface, "AUTO or one of this machine's addresses");
            field(ui, "Address players use:", &mut self.form.server_ip, "AUTO or an address");
            ui.label("The server port must be free and reachable from outside.");

            ui.heading("4. Listserver");
            ui.horizontal(|ui| {
                ui.label("Host:");
                ui.add(egui::TextEdit::singleline(&mut self.form.list_host).hint_text("<listserver-host>"));
                ui.label("Port:");
                ui.add(egui::TextEdit::singleline(&mut self.form.list_port).desired_width(60.0));
                let busy = self.test.is_some();
                if ui.add_enabled(!busy && !self.form.list_host.trim().is_empty(), egui::Button::new("Test connection")).clicked() {
                    if let Ok(port) = self.form.list_port.trim().parse::<u16>() {
                        let host = self.form.list_host.trim().to_string();
                        let (tx, rx) = channel();
                        std::thread::spawn(move || {
                            let _ = tx.send(test_connection(&host, port, Duration::from_secs(5)));
                        });
                        self.test = Some(rx);
                        self.test_result = "testing...".into();
                    }
                }
            });
            if let Some(rx) = &self.test {
                if let Ok(r) = rx.try_recv() {
                    self.test_result = r.to_string();
                    self.test = None;
                } else {
                    ui.ctx().request_repaint_after(Duration::from_millis(200));
                }
            }
            if !self.test_result.is_empty() {
                ui.label(format!("Test connection: {}", self.test_result));
            }
            ui.label("localhost is set to false: logins are then checked by the listserver. Leaving it on would let anyone log in as anyone.");

            ui.heading("5. Router");
            ui.checkbox(&mut self.form.upnp, "Open the port on my router automatically (UPnP)");
            if !self.form.upnp {
                ui.label("Otherwise forward the server's TCP port on your router to this machine's LAN address, and allow it in the firewall.");
            }

            ui.heading("6. Review");
            ui.horizontal(|ui| {
                if ui.button("Review Join a listserver").clicked() {
                    match self.form.answers() {
                        Ok(a) => self.preview = self.folder.as_ref().map(|f| f.preview_public(&a)),
                        Err(e) => self.message = e,
                    }
                }
                if ui.button("Review Back to lab").clicked() {
                    self.preview = self.folder.as_ref().map(ServerFolder::preview_lab);
                }
                if ui.button("Undo last change").clicked() {
                    if let Some(f) = self.folder.as_mut() {
                        self.message = match f.undo() {
                            Ok(b) => format!("Restored from {}.", b.display()),
                            Err(e) => e,
                        };
                    }
                    self.preview = None;
                }
            });
            let mut save = false;
            if let Some(p) = &self.preview {
                if p.diff.is_empty() {
                    ui.label("Nothing to change.");
                }
                egui::Grid::new("diff").striped(true).show(ui, |ui| {
                    for (k, before, after) in &p.diff {
                        ui.monospace(k);
                        ui.monospace(before.as_deref().unwrap_or("(not set)"));
                        ui.label("->");
                        ui.monospace(after.as_deref().unwrap_or("(removed)"));
                        ui.end_row();
                    }
                });
                for f in &p.findings {
                    finding_line(ui, f);
                }
                save = ui
                    .add_enabled(p.may_save() && !p.diff.is_empty(), egui::Button::new(format!("Save {}", options_path.display())))
                    .clicked();
            }
            if save {
                if let (Some(f), Some(p)) = (self.folder.as_mut(), self.preview.take()) {
                    self.message = match f.save(&p) {
                        Ok(Some(b)) => format!("Saved. Backup: {}", b.display()),
                        Ok(None) => "Saved.".into(),
                        Err(e) => e,
                    };
                }
            }

            ui.heading("7. Start and check");
            self.start_and_check(ui);
        });
    }

    fn start_and_check(&mut self, ui: &mut egui::Ui) {
        let Some(folder) = self.folder.as_ref() else {
            return;
        };
        match &mut self.started {
            None => {
                if ui.button("Start the server").clicked() {
                    let spec = ProcessSpec {
                        label: folder.name.clone(),
                        program: folder.world.join(exe_name("gs2emu")),
                        args: vec!["--server".into(), folder.name.clone()],
                        cwd: folder.world.clone(),
                        console: folder.console_path(),
                    };
                    match start(&spec) {
                        Ok(r) => {
                            let mut group = Group::default();
                            group.push(r);
                            self.started = Some(Started {
                                group,
                                console: spec.console,
                                public: folder.profile() != Profile::Lab,
                            });
                        }
                        Err(e) => self.message = e.to_string(),
                    }
                }
            }
            Some(s) => {
                let text = std::fs::read_to_string(&s.console).unwrap_or_default();
                let lights = Lights::read(&text, s.public);
                light_line(ui, "Listening on", &lights.listening);
                light_line(ui, "Listserver", &lights.listserver);
                light_line(ui, "Reachable from outside", &lights.reachable);
                let stopped = !s.group.stopped().is_empty();
                if stopped {
                    ui.label(format!(
                        "The server stopped. Its console: {}",
                        s.console.display()
                    ));
                }
                if ui
                    .button(if stopped { "Close" } else { "Stop the server" })
                    .clicked()
                {
                    s.group.stop_all(Duration::from_secs(5));
                    self.started = None;
                } else {
                    ui.ctx().request_repaint_after(Duration::from_millis(500));
                }
            }
        }
    }
}

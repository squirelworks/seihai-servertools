//! The Host a listserver part of the Server setup page. Rules, checks and
//! writes are in `setup_core::folder::ListserverConfig`; this only holds what
//! the owner typed.

use std::process::Command;

use eframe::egui::{self, RichText};
use setup_core::config::listserver_toml::{AllowedGserver, HostAnswers};
use setup_core::folder::{ListserverConfig, ListserverPreview};
use setup_core::lab::exe_name;

use super::setup::finding_line;

pub struct Host {
    config: Option<ListserverConfig>,
    bind: String,
    gserver_port: String,
    client_ports: String,
    accounts: String,
    gserver_ip: String,
    /// Who may register: (address, HQ password) rows. Empty: any gserver.
    gservers: Vec<(String, String)>,
    max_connections: String,
    silent_list_ms: String,
    site_url: String,
    upgrade_url: String,
    preview: Option<ListserverPreview>,
    new_account: String,
    new_password: String,
    message: String,
}

impl Default for Host {
    fn default() -> Self {
        let d = HostAnswers::default();
        Host {
            config: None,
            bind: d.bind,
            gserver_port: d.gserver_port.to_string(),
            client_ports: d
                .client_ports
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(","),
            accounts: d.accounts,
            gserver_ip: d.gserver_ip,
            gservers: vec![("127.0.0.1".into(), String::new())],
            max_connections: d.max_connections.to_string(),
            silent_list_ms: d.silent_list_after_ms.to_string(),
            site_url: String::new(),
            upgrade_url: String::new(),
            preview: None,
            new_account: String::new(),
            new_password: String::new(),
            message: String::new(),
        }
    }
}

impl Host {
    /// Load `<world>/seihai-listserver.toml` and fill the form from it.
    pub fn open(&mut self, world: &std::path::Path) {
        self.preview = None;
        match ListserverConfig::open(world) {
            Ok(c) => {
                let s = c.toml.settings();
                if !s.accounts.is_empty() {
                    self.accounts = s.accounts.clone();
                }
                self.gserver_port = s.gserver_port.to_string();
                self.max_connections = s.max_connections.to_string();
                if c.toml.get("silent_list_after_ms").is_some() {
                    self.silent_list_ms = s.silent_list_after_ms.to_string();
                }
                if let Some(p) = c.toml.get("client_ports") {
                    self.client_ports = p.trim_matches(['[', ']']).replace(' ', "");
                }
                if !s.gservers.is_empty() || c.toml.get("gservers").is_some() {
                    self.gservers = s
                        .gservers
                        .iter()
                        .map(|g| (g.ip.clone(), g.hq_password.clone()))
                        .collect();
                }
                self.site_url = c.toml.get("site_url").unwrap_or_default();
                self.upgrade_url = c.toml.get("upgrade_url").unwrap_or_default();
                self.message.clear();
                self.config = Some(c);
            }
            Err(e) => {
                self.config = None;
                self.message = e;
            }
        }
    }

    fn answers(&self) -> Result<HostAnswers, String> {
        let client_ports = self
            .client_ports
            .split(',')
            .map(|p| p.trim().parse::<u16>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| {
                format!(
                    "client ports {:?}: whole numbers, comma-separated",
                    self.client_ports
                )
            })?;
        Ok(HostAnswers {
            bind: self.bind.trim().to_string(),
            gserver_port: self
                .gserver_port
                .trim()
                .parse()
                .map_err(|_| "the gserver port is not a number".to_string())?,
            client_ports,
            accounts: self.accounts.trim().to_string(),
            gserver_ip: self.gserver_ip.clone(),
            gservers: self.allow_list()?,
            max_connections: match self.max_connections.trim().parse() {
                Ok(n) if n > 0 => n,
                _ => return Err("most connections: a whole number, at least 1".into()),
            },
            silent_list_after_ms: self
                .silent_list_ms
                .trim()
                .parse()
                .map_err(|_| "silent list: whole milliseconds (0: off)".to_string())?,
            site_url: self.site_url.trim().to_string(),
            upgrade_url: self.upgrade_url.trim().to_string(),
        })
    }

    /// The Who may register rows as the listserver's allow-list. Blank rows
    /// are skipped; an address that does not parse is an error here, so the
    /// review never offers a file the listserver would refuse.
    fn allow_list(&self) -> Result<Vec<AllowedGserver>, String> {
        let mut out: Vec<AllowedGserver> = Vec::new();
        for (ip, pw) in &self.gservers {
            let ip = ip.trim();
            if ip.is_empty() {
                continue;
            }
            if ip.parse::<std::net::IpAddr>().is_err() {
                return Err(format!("Who may register: {ip:?} is not an IP address"));
            }
            if out.iter().any(|g| g.ip == ip) {
                return Err(format!("Who may register: {ip} is listed twice"));
            }
            out.push(AllowedGserver {
                ip: ip.to_string(),
                hq_password: pw.clone(),
            });
        }
        Ok(out)
    }

    /// Run `seihai-listserver add-account`, the password in the child's
    /// environment only.
    fn add_account(&mut self) {
        let Some(c) = &self.config else { return };
        let Some(file) = c.accounts_path(&c.toml) else {
            self.message = "Save an accounts file name first.".into();
            return;
        };
        let world = c.path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        let name = exe_name("seihai-listserver");
        let here = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join(&name)));
        let Some(bin) = [Some(world.join(&name)), here]
            .into_iter()
            .flatten()
            .find(|p| p.is_file())
        else {
            self.message = format!("No {name} next to the world folder or this program.");
            return;
        };
        let out = Command::new(&bin)
            .arg("add-account")
            .arg(&file)
            .arg(self.new_account.trim())
            .arg("--password-env")
            .arg("SEIHAI_SERVERTOOLS_NEW_PASSWORD")
            .env("SEIHAI_SERVERTOOLS_NEW_PASSWORD", &self.new_password)
            .output();
        self.new_password.clear();
        self.message = match out {
            Ok(o) if o.status.success() => {
                format!("Added {} to {}.", self.new_account.trim(), file.display())
            }
            Ok(o) => format!(
                "add-account failed: {}",
                String::from_utf8_lossy(&o.stderr).trim()
            ),
            Err(e) => format!("{}: {e}", bin.display()),
        };
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let Some(c) = &self.config else {
            ui.label("Open a world folder first.");
            if !self.message.is_empty() {
                ui.label(&self.message);
            }
            return;
        };
        let s = c.toml.settings();
        ui.label(format!(
            "{}{}: {}",
            c.path.display(),
            if c.exists { "" } else { " (not created yet)" },
            if s.is_loopback() {
                "this machine only (lab)"
            } else {
                "reachable from other machines"
            }
        ));
        let row = |ui: &mut egui::Ui, label: &str, v: &mut String, hint: &str| {
            ui.horizontal(|ui| {
                ui.label(label);
                ui.add(egui::TextEdit::singleline(v).hint_text(hint));
            });
        };
        ui.heading("1. Safety check");
        ui.label("Other machines can reach this listserver, so it needs an accounts file. Open mode is refused.");
        row(ui, "Accounts file:", &mut self.accounts, "accounts.txt");
        ui.heading("2. Network");
        row(
            ui,
            "Listen on:",
            &mut self.bind,
            "0.0.0.0, :: or one address",
        );
        row(ui, "Gserver port:", &mut self.gserver_port, "14900");
        row(
            ui,
            "Client ports:",
            &mut self.client_ports,
            "14921,14922,14923",
        );
        row(
            ui,
            "Most connections at once:",
            &mut self.max_connections,
            "256 (players asking for the list plus gservers)",
        );
        row(
            ui,
            "Answer a silent client after (ms):",
            &mut self.silent_list_ms,
            "500: real 1.41 rev 1 waits for the list; 0: off",
        );
        ui.heading("3. Who may register");
        ui.label(format!(
            "Only these gserver addresses may register, each after sending its HQ password (hq_password in that gserver's config/adminconfig.txt). Gservers on this machine connect from 127.0.0.1. No rows: any gserver that reaches port {} may register. Still open that port only to these addresses.",
            self.gserver_port
        ));
        let mut remove = None;
        egui::Grid::new("host-gservers").show(ui, |ui| {
            for (i, (ip, pw)) in self.gservers.iter_mut().enumerate() {
                ui.label("Address:");
                ui.add(
                    egui::TextEdit::singleline(ip)
                        .hint_text("127.0.0.1")
                        .desired_width(140.0),
                );
                ui.label("HQ password:");
                ui.add(
                    egui::TextEdit::singleline(pw)
                        .password(true)
                        .desired_width(140.0),
                );
                if ui.button("Remove").clicked() {
                    remove = Some(i);
                }
                ui.end_row();
            }
        });
        if let Some(i) = remove {
            self.gservers.remove(i);
        }
        if ui.button("Add a gserver").clicked() {
            self.gservers.push((String::new(), String::new()));
        }
        ui.heading("4. Addresses handed to clients");
        ui.horizontal(|ui| {
            ui.radio_value(
                &mut self.gserver_ip,
                "sent".to_string(),
                "sent: what each gserver says (use for gservers on this machine)",
            );
        });
        ui.horizontal(|ui| {
            ui.radio_value(
                &mut self.gserver_ip,
                "peer".to_string(),
                "peer: where each gserver connects from",
            );
        });
        ui.heading("5. Site and upgrade URLs (optional)");
        row(ui, "Site URL:", &mut self.site_url, "");
        row(ui, "Upgrade URL:", &mut self.upgrade_url, "");
        ui.heading("6. Router");
        ui.label(format!(
            "Forward TCP {} (players) and {} (only your gservers) to this machine.",
            self.client_ports, self.gserver_port
        ));
        ui.heading("7. Review, save");
        ui.horizontal(|ui| {
            if ui.button("Review Host a listserver").clicked() {
                match self.answers() {
                    Ok(a) => self.preview = self.config.as_ref().map(|c| c.preview_host(&a)),
                    Err(e) => self.message = e,
                }
            }
            if ui.button("Review listserver Back to lab").clicked() {
                self.preview = self.config.as_ref().map(ListserverConfig::preview_lab);
            }
            if ui.button("Undo last change").clicked() {
                if let Some(c) = self.config.as_mut() {
                    self.message = match c.undo() {
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
            egui::Grid::new("host-diff").striped(true).show(ui, |ui| {
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
                .add_enabled(
                    p.may_save() && !p.diff.is_empty(),
                    egui::Button::new("Save"),
                )
                .clicked();
        }
        if save {
            if let (Some(c), Some(p)) = (self.config.as_mut(), self.preview.take()) {
                self.message = match c.save(&p) {
                    Ok(Some(b)) => format!("Saved. Backup: {}", b.display()),
                    Ok(None) => "Saved.".into(),
                    Err(e) => e,
                };
            }
        }
        ui.heading("First account");
        ui.horizontal(|ui| {
            ui.label("Account:");
            ui.add(egui::TextEdit::singleline(&mut self.new_account).desired_width(120.0));
            ui.label("Password:");
            ui.add(
                egui::TextEdit::singleline(&mut self.new_password)
                    .password(true)
                    .desired_width(120.0),
            );
            let ready = !self.new_account.trim().is_empty() && !self.new_password.is_empty();
            if ui
                .add_enabled(ready, egui::Button::new("Add account"))
                .clicked()
            {
                self.add_account();
            }
        });
        ui.heading("8. Your own gservers");
        ui.label("Switch to Join a listserver above and type 127.0.0.1 (or this listserver's address) as the listserver.");
        if !self.message.is_empty() {
            ui.label(RichText::new(&self.message).strong());
        }
    }
}

//! The "Listen on" pick list: this machine's addresses from
//! `setup_core::net::interfaces`, plus the page's own choices (`AUTO`,
//! `0.0.0.0`, `::`). Picking one fills the text field; typing still works.

use eframe::egui;
use setup_core::net::{self, Interface};

pub struct Addresses {
    list: Vec<Interface>,
    error: Option<String>,
}

impl Default for Addresses {
    fn default() -> Self {
        let mut a = Addresses {
            list: Vec::new(),
            error: None,
        };
        a.refresh();
        a
    }
}

impl Addresses {
    /// Read the interface table again (a cable plugged in, a VPN up).
    pub fn refresh(&mut self) {
        match net::interfaces() {
            Ok(list) => {
                self.list = list;
                self.error = None;
            }
            Err(e) => {
                self.list.clear();
                self.error = Some(format!("Could not read this machine's addresses: {e}"));
            }
        }
    }

    /// The choices, as (value, label): `extra` first, then the addresses.
    pub fn choices(&self, extra: &[(&str, &str)]) -> Vec<(String, String)> {
        extra
            .iter()
            .map(|(v, l)| (v.to_string(), format!("{v} ({l})")))
            .chain(self.list.iter().map(|i| (i.addr.to_string(), i.label())))
            .collect()
    }

    /// A "Pick" menu next to a "Listen on" field.
    pub fn pick(
        &mut self,
        ui: &mut egui::Ui,
        id: &str,
        value: &mut String,
        extra: &[(&str, &str)],
    ) {
        let choices = self.choices(extra);
        let mut refresh = false;
        egui::ComboBox::from_id_salt(id)
            .selected_text("Pick")
            .show_ui(ui, |ui| {
                for (v, label) in &choices {
                    ui.selectable_value(value, v.clone(), label);
                }
                if ui.button("Read the addresses again").clicked() {
                    refresh = true;
                }
            });
        if refresh {
            self.refresh();
        }
        if let Some(e) = &self.error {
            ui.label(e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choices_put_the_page_values_first_then_loopback() {
        let a = Addresses::default();
        let c = a.choices(&[("AUTO", "every interface")]);
        assert_eq!(
            c[0],
            ("AUTO".to_string(), "AUTO (every interface)".to_string())
        );
        assert!(
            c.iter()
                .any(|(v, l)| v == "127.0.0.1" && l.contains("this machine only")),
            "{c:?}"
        );
    }
}

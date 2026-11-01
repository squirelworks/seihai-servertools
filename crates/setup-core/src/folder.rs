//! What the setup wizard edits, with load, preview, check, save (with a
//! backup) and undo: one gserver folder ([`ServerFolder`],
//! `<world>/servers/<name>`) and the listserver's settings
//! ([`ListserverConfig`], `<world>/seihai-listserver.toml`). The command line
//! and the window both go through this, so they cannot drift apart.

use std::io;
use std::path::{Path, PathBuf};

use crate::checks::{
    check_gserver, check_listserver, check_listserver_accounts, check_server_folder, may_save,
    Finding, Severity,
};
use crate::config::backup;
use crate::config::listserver_toml::{
    apply_host, listserver_back_to_lab, HostAnswers, ListserverToml,
};
use crate::config::options_txt::OptionsTxt;
use crate::profile::{apply_public, back_to_lab, detect, Profile, PublicAnswers};
use crate::stamp::Stamp;

/// A loaded server folder.
#[derive(Debug, Clone)]
pub struct ServerFolder {
    pub world: PathBuf,
    pub name: String,
    pub dir: PathBuf,
    pub options_path: PathBuf,
    pub options: OptionsTxt,
}

/// A change ready to be reviewed and saved.
#[derive(Debug, Clone)]
pub struct Preview {
    pub target: Profile,
    pub after: OptionsTxt,
    /// `(key, before, after)` for every changed key.
    pub diff: Vec<(String, Option<String>, Option<String>)>,
    pub findings: Vec<Finding>,
}

impl Preview {
    /// True when nothing blocks saving.
    pub fn may_save(&self) -> bool {
        may_save(&self.findings)
    }
}

impl ServerFolder {
    /// Load `<world>/servers/<name>/config/serveroptions.txt`.
    pub fn open(world: &Path, name: &str) -> Result<Self, String> {
        if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\']) {
            return Err(format!("bad server folder name {name:?}"));
        }
        let dir = world.join("servers").join(name);
        let options_path = dir.join("config").join("serveroptions.txt");
        let options = OptionsTxt::load(&options_path)
            .map_err(|e| format!("cannot read {}: {e}", options_path.display()))?;
        Ok(ServerFolder {
            world: world.to_path_buf(),
            name: name.to_string(),
            dir,
            options_path,
            options,
        })
    }

    /// Read the file again (after a save or an undo).
    pub fn reload(&mut self) -> Result<(), String> {
        *self = Self::open(&self.world, &self.name)?;
        Ok(())
    }

    /// Lab or Public, from the current options.
    pub fn profile(&self) -> Profile {
        detect(&self.options)
    }

    /// The folder's own findings (lab accounts, default staff login, default
    /// account) plus the options' findings for its current profile.
    pub fn status_findings(&self) -> Vec<Finding> {
        let mut f = check_gserver(&self.options, self.profile());
        f.extend(check_server_folder(&self.dir));
        f
    }

    fn preview(&self, target: Profile, after: OptionsTxt) -> Preview {
        Preview {
            target,
            diff: self.options.diff(&after),
            findings: check_gserver(&after, target),
            after,
        }
    }

    /// What Join a listserver with these answers would change.
    ///
    /// Blocking folder findings (the default staff login) are included, so
    /// Save stays off until those files are gone. Warnings stay on the
    /// safety check ([`Self::status_findings`]) and do not by themselves
    /// refuse the save.
    pub fn preview_public(&self, answers: &PublicAnswers) -> Preview {
        let mut after = self.options.clone();
        apply_public(&mut after, answers, &Stamp::now().date());
        let mut preview = self.preview(Profile::Public, after);
        preview.findings.extend(
            check_server_folder(&self.dir)
                .into_iter()
                .filter(|f| f.severity == Severity::Block),
        );
        preview
    }

    /// What Back to lab would change.
    pub fn preview_lab(&self) -> Preview {
        let mut after = self.options.clone();
        back_to_lab(&mut after, &Stamp::now().date());
        self.preview(Profile::Lab, after)
    }

    /// Save a preview with a dated backup, then reload. Refuses a preview
    /// with a blocking finding. Returns the backup's path.
    pub fn save(&mut self, preview: &Preview) -> Result<Option<PathBuf>, String> {
        if !preview.may_save() {
            return Err("not saved: a check blocks it".into());
        }
        let b = backup::save(
            &self.options_path,
            preview.after.render().as_bytes(),
            Stamp::now(),
        )
        .map_err(|e: io::Error| format!("cannot save {}: {e}", self.options_path.display()))?;
        self.reload()?;
        Ok(b)
    }

    /// Restore the last backup, then reload. Returns the backup restored.
    pub fn undo(&mut self) -> Result<PathBuf, String> {
        let r = backup::undo(&self.options_path, Stamp::now())
            .map_err(|e| format!("undo {}: {e}", self.options_path.display()))?;
        self.reload()?;
        Ok(r)
    }

    /// Where the gserver's console goes when setup starts it.
    pub fn console_path(&self) -> PathBuf {
        self.dir.join("logs").join("console.txt")
    }
}

/// The listserver's settings file, as the Host a listserver wizard sees it.
#[derive(Debug, Clone)]
pub struct ListserverConfig {
    pub path: PathBuf,
    pub toml: ListserverToml,
    /// False when the file does not exist yet (it is created on save).
    pub exists: bool,
}

/// A listserver change ready to be reviewed and saved.
#[derive(Debug, Clone)]
pub struct ListserverPreview {
    pub after: ListserverToml,
    pub diff: Vec<(String, Option<String>, Option<String>)>,
    pub findings: Vec<Finding>,
}

impl ListserverPreview {
    pub fn may_save(&self) -> bool {
        may_save(&self.findings)
    }
}

impl ListserverConfig {
    /// Load `<world>/seihai-listserver.toml` (a missing file is empty).
    pub fn open(world: &Path) -> Result<Self, String> {
        let path = world.join("seihai-listserver.toml");
        let (text, exists) = match std::fs::read_to_string(&path) {
            Ok(t) => (t, true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => (String::new(), false),
            Err(e) => return Err(format!("cannot read {}: {e}", path.display())),
        };
        let toml = ListserverToml::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(ListserverConfig { path, toml, exists })
    }

    fn folder(&self) -> PathBuf {
        self.path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default()
    }

    /// The accounts file the settings name, resolved from the TOML's folder.
    pub fn accounts_path(&self, toml: &ListserverToml) -> Option<PathBuf> {
        let a = toml.settings().accounts;
        (!a.trim().is_empty()).then(|| self.folder().join(a.trim()))
    }

    /// Findings for settings: [`check_listserver`] plus a missing accounts file.
    pub fn findings(&self, toml: &ListserverToml) -> Vec<Finding> {
        let mut f = check_listserver(&toml.settings());
        if let Some(p) = self.accounts_path(toml) {
            if !p.exists() {
                f.push(Finding {
                    severity: Severity::Warn,
                    key: Some("accounts".into()),
                    text: format!(
                        "{} does not exist yet: nobody can log in until you add the first account (seihai-listserver add-account).",
                        p.display()
                    ),
                });
            } else if !toml.settings().is_loopback() {
                // Lab bind does not publish the file. A public bind does.
                if let Ok(text) = std::fs::read_to_string(&p) {
                    f.extend(check_listserver_accounts(&text));
                }
            }
        }
        f
    }

    fn preview(&self, after: ListserverToml) -> ListserverPreview {
        ListserverPreview {
            diff: self.toml.diff(&after),
            findings: self.findings(&after),
            after,
        }
    }

    /// What Host a listserver with these answers would change.
    pub fn preview_host(&self, answers: &HostAnswers) -> ListserverPreview {
        let mut after = self.toml.clone();
        apply_host(&mut after, answers, &Stamp::now().date());
        self.preview(after)
    }

    /// What Back to lab (bind 127.0.0.1) would change.
    pub fn preview_lab(&self) -> ListserverPreview {
        let mut after = self.toml.clone();
        listserver_back_to_lab(&mut after, &Stamp::now().date());
        self.preview(after)
    }

    /// Save with a dated backup (when the file existed), then reload.
    pub fn save(&mut self, p: &ListserverPreview) -> Result<Option<PathBuf>, String> {
        if !p.may_save() {
            return Err("not saved: a check blocks it".into());
        }
        let b = backup::save(&self.path, p.after.render().as_bytes(), Stamp::now())
            .map_err(|e| format!("cannot save {}: {e}", self.path.display()))?;
        *self = Self::open(&self.folder())?;
        Ok(b)
    }

    /// Restore the last backup, then reload.
    pub fn undo(&mut self) -> Result<PathBuf, String> {
        let r = backup::undo(&self.path, Stamp::now())
            .map_err(|e| format!("undo {}: {e}", self.path.display()))?;
        *self = Self::open(&self.folder())?;
        Ok(r)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const LAB: &str = "name = My Server\nserverinterface = 127.0.0.1\nserverip = 127.0.0.1\nlocalip = 127.0.0.1\nlistip = 127.0.0.1\nlistport = 14900\nupnp = false\nstaff = (Manager),YOURACCOUNT\nlocalhost = true\n";

    fn world() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        let c = d.path().join("servers/default/config");
        fs::create_dir_all(&c).unwrap();
        fs::write(c.join("serveroptions.txt"), LAB).unwrap();
        d
    }

    #[test]
    fn preview_save_undo() {
        let w = world();
        let mut f = ServerFolder::open(w.path(), "default").unwrap();
        assert_eq!(f.profile(), Profile::Lab);
        let answers = PublicAnswers {
            interface: "AUTO".into(),
            server_ip: "AUTO".into(),
            list_host: "127.0.0.1".into(),
            list_port: 14900,
            upnp: false,
            name: "World".into(),
            description: String::new(),
            url: String::new(),
            staff_account: "owner".into(),
        };
        let p = f.preview_public(&answers);
        assert!(p.may_save(), "{:?}", p.findings);
        assert!(p.diff.iter().any(|(k, _, _)| k == "localhost"));
        f.save(&p).unwrap().expect("a backup");
        assert_eq!(f.profile(), Profile::Public);
        f.undo().unwrap();
        assert_eq!(f.profile(), Profile::Lab);
        assert_eq!(fs::read_to_string(&f.options_path).unwrap(), LAB);
    }

    #[test]
    fn public_preview_blocks_a_default_staff_account_file() {
        let w = world();
        let acc = w.path().join("servers/default/accounts");
        fs::create_dir_all(&acc).unwrap();
        fs::write(acc.join("super.txt"), "PASSWORD super\n").unwrap();
        let mut f = ServerFolder::open(w.path(), "default").unwrap();
        let answers = PublicAnswers {
            interface: "AUTO".into(),
            server_ip: "AUTO".into(),
            list_host: "127.0.0.1".into(),
            list_port: 14900,
            upnp: false,
            name: "World".into(),
            description: String::new(),
            url: String::new(),
            staff_account: "owner".into(),
        };
        let p = f.preview_public(&answers);
        assert!(!p.may_save(), "{:?}", p.findings);
        assert!(p
            .findings
            .iter()
            .any(|f| { f.severity == Severity::Block && f.text.contains("super.txt") }));
        assert!(f.save(&p).is_err());
    }

    #[test]
    fn host_preview_blocks_a_plaintext_default_staff_password() {
        let w = world();
        fs::write(w.path().join("accounts.txt"), "super super\n").unwrap();
        let c = ListserverConfig::open(w.path()).unwrap();
        let p = c.preview_host(&HostAnswers::default());
        assert!(!p.may_save(), "{:?}", p.findings);
        assert!(p.findings.iter().any(|f| {
            f.severity == Severity::Block
                && f.text.contains("super")
                && !f.text.contains("super super")
        }));
        let back = c.preview_lab();
        assert!(
            back.may_save(),
            "back to lab is not a publish: {:?}",
            back.findings
        );
    }

    #[test]
    fn blocked_preview_is_not_saved() {
        let w = world();
        let mut f = ServerFolder::open(w.path(), "default").unwrap();
        let mut after = f.options.clone();
        after.set("serverinterface", "AUTO", "2026-10-31");
        let p = f.preview(Profile::Lab, after);
        assert!(!p.may_save());
        assert!(f.save(&p).is_err());
        assert_eq!(fs::read_to_string(&f.options_path).unwrap(), LAB);
    }

    #[test]
    fn listserver_host_save_and_back() {
        let w = world();
        let mut c = ListserverConfig::open(w.path()).unwrap();
        assert!(!c.exists);
        // Open mode on outside interfaces is refused.
        let open = HostAnswers {
            accounts: String::new(),
            ..HostAnswers::default()
        };
        assert!(!c.preview_host(&open).may_save());
        let p = c.preview_host(&HostAnswers::default());
        assert!(p.may_save(), "{:?}", p.findings);
        assert!(p
            .findings
            .iter()
            .any(|f| f.text.contains("does not exist yet")));
        assert_eq!(c.save(&p).unwrap(), None, "no backup for a new file");
        assert!(c.exists);
        assert_eq!(c.toml.settings().bind, "0.0.0.0");
        let p = c.preview_lab();
        c.save(&p).unwrap().expect("a backup");
        assert!(c.toml.settings().is_loopback());
        c.undo().unwrap();
        assert_eq!(c.toml.settings().bind, "0.0.0.0");
    }

    #[test]
    fn bad_names() {
        let w = world();
        for n in ["", "..", "a/b", "nope"] {
            assert!(ServerFolder::open(w.path(), n).is_err(), "{n}");
        }
    }
}

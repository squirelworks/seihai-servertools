//! One gserver folder (`<world>/servers/<name>`) as the setup wizard sees
//! it: load its options, preview a change, check it, save it with a backup,
//! undo. The command line and the window both go through this, so they
//! cannot drift apart.

use std::io;
use std::path::{Path, PathBuf};

use crate::checks::{check_gserver, check_server_folder, may_save, Finding};
use crate::config::backup;
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

    /// The folder's own findings (lab accounts, default account) plus the
    /// options' findings for its current profile.
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
    pub fn preview_public(&self, answers: &PublicAnswers) -> Preview {
        let mut after = self.options.clone();
        apply_public(&mut after, answers, &Stamp::now().date());
        self.preview(Profile::Public, after)
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
    fn bad_names() {
        let w = world();
        for n in ["", "..", "a/b", "nope"] {
            assert!(ServerFolder::open(w.path(), n).is_err(), "{n}");
        }
    }
}

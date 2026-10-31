//! Safety check and the combinations the setup wizard refuses to save.
//!
//! The rules come from `docs/design/go-public.md` ("Safety check" and
//! "Combinations the GUI refuses to save"). Each has a test. The command
//! line and the window both show these findings and both refuse to save
//! while any is [`Severity::Block`] ([`may_save`]).

use std::fs;
use std::path::Path;

use crate::config::listserver_toml::ListserverLab;
use crate::config::options_txt::OptionsTxt;
use crate::profile::{is_loopback, is_true, Profile};

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Shown for information.
    Info,
    /// Shown; saving is still allowed.
    Warn,
    /// Saving is refused until it is fixed.
    Block,
}

/// One thing the check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub severity: Severity,
    /// The key the finding is about, if one.
    pub key: Option<String>,
    /// One plain sentence for the owner.
    pub text: String,
}

fn finding(severity: Severity, key: &str, text: impl Into<String>) -> Finding {
    Finding {
        severity,
        key: (!key.is_empty()).then(|| key.to_string()),
        text: text.into(),
    }
}

fn is_placeholder(v: &str) -> bool {
    let v = v.trim();
    v.starts_with('<') && v.ends_with('>')
}

/// Check a gserver's options before they are saved as `target`.
///
/// The refused combinations apply to every target; the safety check
/// (owner's account, server name, real listserver address) applies when
/// going public.
pub fn check_gserver(options: &OptionsTxt, target: Profile) -> Vec<Finding> {
    use Severity::*;
    let mut out = Vec::new();
    let interface = options.get("serverinterface");
    let localhost = is_true(options.get("localhost"));
    let listip = options.get("listip").unwrap_or("").trim();

    // Refused combinations.
    if localhost && !is_loopback(interface) {
        out.push(finding(
            Block,
            "localhost",
            "localhost = true lets anyone log in as anyone; it is only safe while serverinterface is 127.0.0.1.",
        ));
    }
    if !localhost && listip.is_empty() {
        out.push(finding(
            Block,
            "listip",
            "Logins are checked by the listserver (localhost = false), but no listserver address is set.",
        ));
    }
    if is_true(options.get("upnp")) && is_loopback(interface) {
        out.push(finding(
            Block,
            "upnp",
            "UPnP opens a router port, but the server only listens on 127.0.0.1; turn one of them off.",
        ));
    }

    if target == Profile::Public || target == Profile::HostListserver {
        if is_loopback(interface) {
            out.push(finding(
                Block,
                "serverinterface",
                "A public server must listen on AUTO or a real interface, not 127.0.0.1.",
            ));
        }
        let staff = options.get("staff").unwrap_or("");
        if staff
            .split(',')
            .any(|s| s.trim().eq_ignore_ascii_case("YOURACCOUNT"))
        {
            out.push(finding(
                Block,
                "staff",
                "staff still says YOURACCOUNT: put your own account there.",
            ));
        }
        if is_placeholder(listip) {
            out.push(finding(
                Block,
                "listip",
                format!("{listip} is a placeholder: type the listserver's address."),
            ));
        }
        match options.get("name").map(str::trim) {
            None | Some("") | Some("My Server") => out.push(finding(
                Warn,
                "name",
                "The server is still called \"My Server\"; players see this name in the list.",
            )),
            _ => {}
        }
        if localhost {
            // Already blocked above unless the interface is loopback, which
            // is blocked too; say the reason in going-public terms as well.
            out.push(finding(
                Info,
                "localhost",
                "Going public sets localhost = false: logins are then checked by the listserver.",
            ));
        }
    }
    out
}

/// Check a server folder (`servers/<name>`) before going public: accounts
/// left from lab testing, and what new players get.
pub fn check_server_folder(dir: &Path) -> Vec<Finding> {
    let mut out = Vec::new();
    let accounts = dir.join("accounts");
    let mut names: Vec<String> = fs::read_dir(&accounts)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .filter(|e| e.path().is_file())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    let default = names
        .iter()
        .position(|n| n.eq_ignore_ascii_case("defaultaccount.txt"))
        .map(|i| names.remove(i));
    if !names.is_empty() {
        out.push(finding(
            Severity::Warn,
            "",
            format!(
                "{} account file(s) in accounts/ (lab test accounts?): {}. Remove the ones you do not want on a public server.",
                names.len(),
                names.join(", ")
            ),
        ));
    }
    if let Some(d) = default {
        out.push(finding(
            Severity::Info,
            "",
            format!("New players start from accounts/{d}: check its start level and items."),
        ));
    }
    out
}

/// Check listserver settings before saving them.
pub fn check_listserver(settings: &ListserverLab) -> Vec<Finding> {
    let mut out = Vec::new();
    if settings.is_loopback() {
        return out;
    }
    if settings.accounts.trim().is_empty() {
        out.push(finding(
            Severity::Block,
            "accounts",
            "A listserver other machines can reach needs an accounts file; open mode lets every account and password in.",
        ));
    }
    out.push(finding(
        Severity::Warn,
        "gserver_port",
        format!(
            "Any program that reaches port {} can register a server and check passwords: open it only to your own gservers (the listserver has no allow-list yet).",
            settings.gserver_port
        ),
    ));
    if settings.gserver_ip == "peer" {
        out.push(finding(
            Severity::Info,
            "gserver_ip",
            "gserver_ip = \"peer\": a gserver on this machine is listed as 127.0.0.1, which players elsewhere cannot use. Use \"sent\" and set that gserver's serverip.",
        ));
    }
    out
}

/// True when nothing in `findings` blocks saving.
pub fn may_save(findings: &[Finding]) -> bool {
    findings.iter().all(|f| f.severity != Severity::Block)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(text: &str) -> OptionsTxt {
        OptionsTxt::parse(text)
    }

    fn keys(f: &[Finding], sev: Severity) -> Vec<String> {
        f.iter()
            .filter(|x| x.severity == sev)
            .filter_map(|x| x.key.clone())
            .collect()
    }

    const LAB: &str = "name = My Server\nserverinterface = 127.0.0.1\nlistip = 127.0.0.1\nupnp = false\nstaff = (Manager),YOURACCOUNT\nlocalhost = true\n";
    const PUBLIC: &str = "name = Owner's World\nserverinterface = AUTO\nlistip = <listserver-host>\nupnp = false\nstaff = (Manager),owner\nlocalhost = false\n";

    #[test]
    fn the_lab_defaults_are_fine_as_lab() {
        assert!(check_gserver(&opts(LAB), Profile::Lab).is_empty());
    }

    #[test]
    fn refused_localhost_true_off_loopback() {
        let f = check_gserver(
            &opts("serverinterface = AUTO\nlocalhost = true\n"),
            Profile::Lab,
        );
        assert_eq!(keys(&f, Severity::Block), ["localhost"]);
        assert!(!may_save(&f));
    }

    #[test]
    fn refused_localhost_false_without_listserver() {
        for text in [
            "serverinterface = 127.0.0.1\nlocalhost = false\n",
            "serverinterface = 127.0.0.1\nlocalhost = false\nlistip = \n",
        ] {
            let f = check_gserver(&opts(text), Profile::Lab);
            assert_eq!(keys(&f, Severity::Block), ["listip"], "{text}");
        }
    }

    #[test]
    fn refused_upnp_on_loopback() {
        let f = check_gserver(
            &opts(&LAB.replace("upnp = false", "upnp = true")),
            Profile::Lab,
        );
        assert_eq!(keys(&f, Severity::Block), ["upnp"]);
    }

    #[test]
    fn going_public_blocks_youraccount_placeholder_and_loopback() {
        let f = check_gserver(&opts(LAB), Profile::Public);
        let blocks = keys(&f, Severity::Block);
        assert!(blocks.contains(&"staff".to_string()), "{blocks:?}");
        assert!(
            blocks.contains(&"serverinterface".to_string()),
            "{blocks:?}"
        );
        assert_eq!(keys(&f, Severity::Warn), ["name"]);

        let f = check_gserver(&opts(PUBLIC), Profile::Public);
        assert_eq!(keys(&f, Severity::Block), ["listip"], "placeholder address");
    }

    #[test]
    fn a_filled_in_public_server_may_save() {
        let text = PUBLIC.replace("<listserver-host>", "127.0.0.1");
        let f = check_gserver(&opts(&text), Profile::Public);
        assert!(may_save(&f), "{f:?}");
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn lab_accounts_are_listed_and_the_default_account_is_shown() {
        let dir = tempfile::tempdir().unwrap();
        let acc = dir.path().join("accounts");
        fs::create_dir_all(&acc).unwrap();
        for n in ["test205.txt", "defaultaccount.txt", "test1.txt"] {
            fs::write(acc.join(n), "").unwrap();
        }
        let f = check_server_folder(dir.path());
        assert_eq!(f.len(), 2);
        assert!(
            f[0].text.contains("test1.txt, test205.txt"),
            "{}",
            f[0].text
        );
        assert_eq!(f[1].severity, Severity::Info);
        assert!(check_server_folder(&dir.path().join("none")).is_empty());
    }

    #[test]
    fn public_listserver_needs_accounts() {
        let lab = ListserverLab::default();
        assert!(check_listserver(&lab).is_empty());
        let open = ListserverLab {
            bind: "0.0.0.0".into(),
            ..ListserverLab::default()
        };
        let f = check_listserver(&open);
        assert_eq!(keys(&f, Severity::Block), ["accounts"]);
        let closed = ListserverLab {
            accounts: "accounts.txt".into(),
            gserver_ip: "sent".into(),
            ..open
        };
        let f = check_listserver(&closed);
        assert!(may_save(&f));
        assert_eq!(keys(&f, Severity::Warn), ["gserver_port"]);
    }
}

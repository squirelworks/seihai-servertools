//! Safety check and the combinations the setup wizard refuses to save.
//!
//! The rules come from `docs/design/go-public.md` ("Safety check" and
//! "Combinations the GUI refuses to save"). Each has a test. Publishing
//! also refuses a default staff login (`YOURACCOUNT`, password `super`)
//! and warns about open lab accounts and bot-farm account files. The
//! command line and the window both show these findings and both refuse
//! to save while any is [`Severity::Block`] ([`may_save`]).

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
        if staff
            .split(',')
            .any(|s| s.trim().eq_ignore_ascii_case("super"))
        {
            out.push(finding(
                Block,
                "staff",
                "staff still lists super, the default staff account (password super): replace it with your own account.",
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
/// left from lab testing, the default staff login, and what new players get.
pub fn check_server_folder(dir: &Path) -> Vec<Finding> {
    let mut out = Vec::new();
    let accounts = dir.join("accounts");
    let mut files: Vec<(String, std::path::PathBuf)> = fs::read_dir(&accounts)
        .map(|rd| {
            rd.filter_map(Result::ok)
                .filter(|e| e.path().is_file())
                .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
                .collect()
        })
        .unwrap_or_default();
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let mut staff = Vec::new();
    let mut open = Vec::new();
    let mut bots = Vec::new();
    let mut other = Vec::new();
    let mut defaults = Vec::new();
    for (name, path) in files {
        let stem = account_stem(&name);
        if is_default_template(stem) {
            defaults.push(name);
            continue;
        }
        // The NPC server's own account. It is not a player leftover.
        if is_npc_account(stem) {
            continue;
        }
        match account_kind(stem, password_field_in(&path).as_deref()) {
            AccountKind::Staff => staff.push(name),
            AccountKind::OpenLab => open.push(name),
            AccountKind::BotFarm => bots.push(name),
            AccountKind::Other => other.push(name),
        }
    }
    if !staff.is_empty() {
        out.push(finding(
            Severity::Block,
            "",
            format!(
                "accounts/ still has the default staff login in: {}. Remove those files, or give them a password other than super, before publishing.",
                staff.join(", ")
            ),
        ));
    }
    if !open.is_empty() {
        out.push(finding(
            Severity::Warn,
            "",
            format!(
                "{} open lab account(s) in accounts/: {}. Remove them before publishing; they are shared lab logins or use the account name as the password.",
                open.len(),
                open.join(", ")
            ),
        ));
    }
    if !bots.is_empty() {
        out.push(finding(
            Severity::Warn,
            "",
            format!(
                "{} bot-farm account file(s) in accounts/: {}. Remove them before publishing.",
                bots.len(),
                bots.join(", ")
            ),
        ));
    }
    if !other.is_empty() {
        out.push(finding(
            Severity::Warn,
            "",
            format!(
                "{} account file(s) in accounts/ (lab test accounts?): {}. Remove the ones you do not want on a public server.",
                other.len(),
                other.join(", ")
            ),
        ));
    }
    if !defaults.is_empty() {
        out.push(finding(
            Severity::Info,
            "",
            format!(
                "New players start from accounts/{}: check its start level and items.",
                defaults.join(", ")
            ),
        ));
    }
    out
}

/// Check a listserver accounts file (`account secret` per line) before that
/// listserver is reachable from other machines.
///
/// A hashed line is fine. A plaintext `super` password, or the lab staff
/// account stored in the clear, blocks publishing. Open lab names, bot-farm
/// names and any other plaintext password are warnings. The password itself
/// is never copied into the finding.
pub fn check_listserver_accounts(text: &str) -> Vec<Finding> {
    let mut staff = Vec::new();
    let mut hashed_staff = Vec::new();
    let mut open = Vec::new();
    let mut bots = Vec::new();
    let mut plain = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let (Some(account), Some(secret)) = (parts.next(), parts.next()) else {
            continue;
        };
        let hashed = is_stored_hash(secret);
        if !hashed
            && (secret.eq_ignore_ascii_case("super")
                || account.eq_ignore_ascii_case("super")
                || account.eq_ignore_ascii_case("youraccount"))
        {
            staff.push(account.to_string());
        } else if hashed
            && (account.eq_ignore_ascii_case("super")
                || account.eq_ignore_ascii_case("youraccount"))
        {
            hashed_staff.push(account.to_string());
        } else if looks_like_open_lab(account) || (!hashed && secret.eq_ignore_ascii_case(account))
        {
            open.push(account.to_string());
        } else if looks_like_bot_farm(account) {
            bots.push(account.to_string());
        } else if !hashed {
            plain.push(account.to_string());
        }
    }
    for names in [
        &mut staff,
        &mut hashed_staff,
        &mut open,
        &mut bots,
        &mut plain,
    ] {
        names.sort();
    }
    let mut out = Vec::new();
    if !staff.is_empty() {
        out.push(finding(
            Severity::Block,
            "accounts",
            format!(
                "The listserver accounts file still has a default staff login in the clear ({}). Remove those lines and add the accounts again with a hash before publishing.",
                staff.join(", ")
            ),
        ));
    }
    if !hashed_staff.is_empty() {
        out.push(finding(
            Severity::Warn,
            "accounts",
            format!(
                "The listserver accounts file still lists the default staff name: {}. If that password is still super, change it before publishing.",
                hashed_staff.join(", ")
            ),
        ));
    }
    if !open.is_empty() {
        out.push(finding(
            Severity::Warn,
            "accounts",
            format!(
                "The listserver accounts file still has open lab accounts: {}. Remove them before publishing.",
                open.join(", ")
            ),
        ));
    }
    if !bots.is_empty() {
        out.push(finding(
            Severity::Warn,
            "accounts",
            format!(
                "The listserver accounts file still has bot-farm accounts: {}. Remove them before publishing.",
                bots.join(", ")
            ),
        ));
    }
    if !plain.is_empty() {
        out.push(finding(
            Severity::Warn,
            "accounts",
            format!(
                "The listserver accounts file still has plaintext passwords for: {}. Add those accounts again so the passwords are hashed before publishing.",
                plain.join(", ")
            ),
        ));
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AccountKind {
    Staff,
    OpenLab,
    BotFarm,
    Other,
}

fn account_stem(file_name: &str) -> &str {
    let n = file_name.len();
    if n >= 4 && file_name[n - 4..].eq_ignore_ascii_case(".txt") {
        &file_name[..n - 4]
    } else {
        file_name
    }
}

fn is_default_template(stem: &str) -> bool {
    let s = stem.to_ascii_lowercase();
    s == "defaultaccount" || s == "(defaultaccount)"
}

fn is_npc_account(stem: &str) -> bool {
    let s = stem.to_ascii_lowercase();
    s == "npcserver" || s == "(npcserver)"
}

fn account_kind(stem: &str, password: Option<&str>) -> AccountKind {
    let password_is_super = password.is_some_and(|p| p.eq_ignore_ascii_case("super"));
    let password_is_name = password.is_some_and(|p| !p.is_empty() && p.eq_ignore_ascii_case(stem));
    if stem.eq_ignore_ascii_case("youraccount")
        || stem.eq_ignore_ascii_case("super")
        || password_is_super
    {
        AccountKind::Staff
    } else if looks_like_open_lab(stem) || password_is_name {
        AccountKind::OpenLab
    } else if looks_like_bot_farm(stem) {
        AccountKind::BotFarm
    } else {
        AccountKind::Other
    }
}

/// `guest`, `test`, or `test` plus digits (`test1`, `test205`).
fn looks_like_open_lab(stem: &str) -> bool {
    let s = stem.to_ascii_lowercase();
    if matches!(s.as_str(), "guest" | "test") {
        return true;
    }
    let Some(rest) = s.strip_prefix("test") else {
        return false;
    };
    !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
}

/// Obvious bot-farm file names: `bot`, `afk`, `farm`, `miner`, `macro`,
/// optionally followed by digits or `-`/`_`, plus a few joined names
/// (`afkbot`, `botfarm`). `both` and `farming` do not match.
fn looks_like_bot_farm(stem: &str) -> bool {
    let s = stem.to_ascii_lowercase();
    if matches!(s.as_str(), "afkbot" | "botfarm" | "idlebot" | "autofarm") {
        return true;
    }
    for prefix in ["bot", "afk", "farm", "miner", "macro"] {
        if let Some(rest) = s.strip_prefix(prefix) {
            if rest.is_empty()
                || rest.starts_with(|c: char| c.is_ascii_digit() || c == '_' || c == '-')
            {
                return true;
            }
        }
    }
    false
}

fn is_stored_hash(secret: &str) -> bool {
    secret.starts_with("$argon2") || secret.to_ascii_lowercase().starts_with("pbkdf2-")
}

/// The first `PASSWORD` token in an account file.
///
/// Only the start of the file is read. A stored hash is returned as-is and
/// does not match the default password. An empty field is returned as empty:
/// a listserver-backed account often has no local password, and that is not
/// an open login.
fn password_field_in(path: &Path) -> Option<String> {
    use std::io::Read;
    let mut file = fs::File::open(path).ok()?;
    let mut buf = [0u8; 64 * 1024];
    let n = file.read(&mut buf).ok()?;
    password_field(&String::from_utf8_lossy(&buf[..n]))
}

fn password_field(text: &str) -> Option<String> {
    for line in text.lines() {
        let line = line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let (key, value) = if let Some((k, v)) = line.split_once('=') {
            (k.trim(), v.trim())
        } else if let Some((k, v)) = line.split_once(char::is_whitespace) {
            (k.trim(), v.trim())
        } else {
            continue;
        };
        if key.eq_ignore_ascii_case("PASSWORD") {
            let token = value.split_whitespace().next().unwrap_or("");
            return Some(token.to_string());
        }
    }
    None
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
        assert_eq!(f[0].severity, Severity::Warn);
        assert!(f[0].text.contains("open lab"), "{}", f[0].text);
        assert!(
            f[0].text.contains("test1.txt, test205.txt"),
            "{}",
            f[0].text
        );
        assert!(may_save(&f));
        assert_eq!(f[1].severity, Severity::Info);
        assert!(f[1].text.contains("defaultaccount.txt"), "{}", f[1].text);
        assert!(check_server_folder(&dir.path().join("none")).is_empty());
    }

    #[test]
    fn stock_template_names_are_not_lab_leftovers() {
        let dir = tempfile::tempdir().unwrap();
        let acc = dir.path().join("accounts");
        fs::create_dir_all(&acc).unwrap();
        fs::write(acc.join("(defaultaccount).txt"), "GRACC001\n").unwrap();
        fs::write(acc.join("(npcserver).txt"), "GRACC001\n").unwrap();
        let f = check_server_folder(dir.path());
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Info);
        assert!(f[0].text.contains("(defaultaccount).txt"), "{}", f[0].text);
        assert!(!f[0].text.contains("npcserver"), "{}", f[0].text);
    }

    #[test]
    fn default_staff_open_lab_and_bot_farm_accounts() {
        let dir = tempfile::tempdir().unwrap();
        let acc = dir.path().join("accounts");
        fs::create_dir_all(&acc).unwrap();
        let write = |name: &str, body: &str| fs::write(acc.join(name), body).unwrap();
        write("YOURACCOUNT.txt", "");
        write("super.txt", "PASSWORD something-else\n");
        write("owner.txt", "PASSWORD super\n");
        write("carol.txt", "PASSWORD pbkdf2-sha256:1:aa:bb\n");
        write("guest.txt", "");
        write("keeper.txt", "PASSWORD keeper\n");
        write("bot1.txt", "");
        write("afk-2.txt", "");
        write("botfarm.txt", "");
        write("both.txt", "");
        write("farming.txt", "");
        write("alice.txt", "NICK alice\n");
        let f = check_server_folder(dir.path());
        assert!(!may_save(&f), "{f:?}");
        let block = f.iter().find(|x| x.severity == Severity::Block).unwrap();
        assert!(
            block.text.contains("YOURACCOUNT.txt, owner.txt, super.txt"),
            "{}",
            block.text
        );
        let open = f.iter().find(|x| x.text.contains("open lab")).unwrap();
        assert_eq!(open.severity, Severity::Warn);
        assert!(open.text.contains("guest.txt, keeper.txt"), "{}", open.text);
        let bots = f.iter().find(|x| x.text.contains("bot-farm")).unwrap();
        assert!(
            bots.text.contains("afk-2.txt, bot1.txt, botfarm.txt"),
            "{}",
            bots.text
        );
        let other = f
            .iter()
            .find(|x| x.text.contains("lab test accounts?"))
            .unwrap();
        assert!(
            other
                .text
                .contains("alice.txt, both.txt, carol.txt, farming.txt"),
            "{}",
            other.text
        );
        assert!(may_save(
            &f.iter()
                .filter(|x| x.severity != Severity::Block)
                .cloned()
                .collect::<Vec<_>>()
        ));
    }

    #[test]
    fn default_staff_name_super_blocks_only_when_public() {
        let lab = LAB.replace("YOURACCOUNT", "super");
        assert!(
            check_gserver(&opts(&lab), Profile::Lab).is_empty(),
            "lab may keep the name until publish"
        );
        let public = PUBLIC
            .replace("<listserver-host>", "127.0.0.1")
            .replace("staff = (Manager),owner", "staff = (Manager),super");
        let f = check_gserver(&opts(&public), Profile::Public);
        assert_eq!(keys(&f, Severity::Block), ["staff"]);
        assert!(f[0].text.contains("password super"), "{}", f[0].text);
        assert!(!may_save(&f));
        let supervisor = public.replace("staff = (Manager),super", "staff = (Manager),supervisor");
        assert!(
            check_gserver(&opts(&supervisor), Profile::Public).is_empty(),
            "supervisor is not the default staff account"
        );
    }

    #[test]
    fn listserver_accounts_flag_default_staff_and_leftovers() {
        assert!(check_listserver_accounts("").is_empty());
        assert!(check_listserver_accounts("# comment\n").is_empty());
        let hashed = "alice $argon2id$v=19$m=1,t=1,p=1$c2FsdA$aGFzaA\n";
        assert!(
            check_listserver_accounts(hashed).is_empty(),
            "a hashed account is not a leftover"
        );
        let hashed_super = "super $argon2id$v=19$m=1,t=1,p=1$c2FsdA$aGFzaA\n";
        let f = check_listserver_accounts(hashed_super);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Warn);
        assert!(may_save(&f), "{f:?}");
        assert!(!f[0].text.contains("$argon2"), "the hash is not repeated");

        let text = "\
# lab
super super
YOURACCOUNT lab-only
guest guest
test205 secret
keeper keeper
bot12 x
afkbot x
bob hunter2
alice $argon2id$v=19$m=1,t=1,p=1$c2FsdA$aGFzaA
";
        let f = check_listserver_accounts(text);
        assert!(!may_save(&f), "{f:?}");
        let block = f.iter().find(|x| x.severity == Severity::Block).unwrap();
        assert!(block.text.contains("YOURACCOUNT, super"), "{}", block.text);
        assert!(
            !block.text.contains("lab-only") && !block.text.contains("hunter2"),
            "passwords stay out of the finding: {}",
            block.text
        );
        let open = f.iter().find(|x| x.text.contains("open lab")).unwrap();
        assert!(
            open.text.contains("guest, keeper, test205"),
            "{}",
            open.text
        );
        let bots = f.iter().find(|x| x.text.contains("bot-farm")).unwrap();
        assert!(bots.text.contains("afkbot, bot12"), "{}", bots.text);
        let plain = f.iter().find(|x| x.text.contains("bob")).unwrap();
        assert!(plain.text.contains("plaintext"), "{}", plain.text);
        assert!(!plain.text.contains("hunter2"), "{}", plain.text);
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

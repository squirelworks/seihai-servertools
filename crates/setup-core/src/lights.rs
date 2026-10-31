//! The three lights of the wizard's "Start and check" step, read from a
//! gserver's console output.
//!
//! The lines matched are what the fork's `gs2emu` prints, as seen in lab
//! runs (a fact about its output, not its code):
//!
//! | Console line | Meaning |
//! |---|---|
//! | `Listening on: <ip>:<port>.` | the player socket is up |
//! | `Could not initialize listening socket` / `Could not connect listening socket` | the port could not be opened |
//! | `listserver - Connected to <ip>:<port>.` | linked to the listserver |
//! | `listserver - Disconnected.` | the link dropped |
//! | `Could not connect listserver socket` / `listserver - [Error] <text>` | the link failed |
//!
//! The third light, "reachable from outside", cannot be proven from this
//! machine (NAT, firewalls); it always says so.

/// State of one light.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Light {
    /// Nothing seen yet.
    Waiting,
    /// Good, with what was seen.
    Ok(String),
    /// Working but not what the profile wants (for example loopback only
    /// on a public server).
    Warn(String),
    /// Failed, with the server's words.
    Bad(String),
    /// Cannot be known from here.
    Unknown(String),
}

/// The three lights.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lights {
    pub listening: Light,
    pub listserver: Light,
    pub reachable: Light,
}

impl Lights {
    /// Read a console's text so far. `public` is true when the folder is set
    /// up for the public: then listening on loopback only is a warning.
    pub fn read(console: &str, public: bool) -> Self {
        let mut listening = Light::Waiting;
        let mut listserver = Light::Waiting;
        for line in console.lines() {
            let line = line.trim();
            if let Some(rest) = after(line, "Listening on: ") {
                let addr = rest.trim_end_matches('.').to_string();
                listening = if public && addr.starts_with("127.") {
                    Light::Warn(format!("{addr}: this machine only"))
                } else {
                    Light::Ok(addr)
                };
            } else if line.contains("Could not initialize listening socket")
                || line.contains("Could not connect listening socket")
            {
                listening = Light::Bad(line.to_string());
            } else if let Some(rest) = after(line, "listserver - Connected to ") {
                listserver = Light::Ok(format!("connected to {}", rest.trim_end_matches('.')));
            } else if line.contains("listserver - Disconnected") {
                listserver = Light::Bad("the listserver link dropped".into());
            } else if line.contains("Could not connect listserver socket")
                || line.contains("Could not initialize listserver socket")
            {
                listserver = Light::Bad(line.to_string());
            } else if let Some(rest) = after(line, "listserver - [Error] ") {
                listserver = Light::Bad(rest.to_string());
            }
        }
        let reachable = Light::Unknown(
            "cannot be checked from this machine: ask someone elsewhere to connect".into(),
        );
        Lights {
            listening,
            listserver,
            reachable,
        }
    }

    /// True when the first two lights have settled (not waiting).
    pub fn settled(&self) -> bool {
        self.listening != Light::Waiting && self.listserver != Light::Waiting
    }
}

fn after<'a>(line: &'a str, marker: &str) -> Option<&'a str> {
    line.find(marker).map(|i| &line[i + marker.len()..])
}

impl std::fmt::Display for Light {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Light::Waiting => write!(f, "[ .. ] waiting"),
            Light::Ok(t) => write!(f, "[ ok ] {t}"),
            Light::Warn(t) => write!(f, "[warn] {t}"),
            Light::Bad(t) => write!(f, "[FAIL] {t}"),
            Light::Unknown(t) => write!(f, "[ ?? ] {t}"),
        }
    }
}

impl std::fmt::Display for Lights {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Listening on:            {}", self.listening)?;
        writeln!(f, "Listserver:              {}", self.listserver)?;
        write!(f, "Reachable from outside:  {}", self.reachable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "Loading server configuration.\nInitializing player listen socket.\nListening on: 0.0.0.0:14802.\nInitializing listserver socket.\nlistserver - Connected to 127.0.0.1:14900.\n";

    #[test]
    fn nothing_yet() {
        let l = Lights::read("Loading server configuration.\n", true);
        assert_eq!(l.listening, Light::Waiting);
        assert!(!l.settled());
    }

    #[test]
    fn good_start() {
        let l = Lights::read(GOOD, true);
        assert_eq!(l.listening, Light::Ok("0.0.0.0:14802".into()));
        assert_eq!(
            l.listserver,
            Light::Ok("connected to 127.0.0.1:14900".into())
        );
        assert!(matches!(l.reachable, Light::Unknown(_)));
        assert!(l.settled());
    }

    #[test]
    fn loopback_on_a_public_server_warns() {
        let text = GOOD.replace("0.0.0.0", "127.0.0.1");
        assert!(matches!(
            Lights::read(&text, true).listening,
            Light::Warn(_)
        ));
        assert!(matches!(Lights::read(&text, false).listening, Light::Ok(_)));
    }

    #[test]
    fn failures_and_the_last_state_wins() {
        let l = Lights::read(
            "** [Error] Could not initialize listening socket.\n[Error] Could not connect listserver socket.\n",
            true,
        );
        assert!(matches!(l.listening, Light::Bad(_)));
        assert!(matches!(l.listserver, Light::Bad(_)));
        let l = Lights::read(&format!("{GOOD}listserver - Disconnected.\n"), true);
        assert!(matches!(l.listserver, Light::Bad(_)));
        let l = Lights::read(
            &format!(
                "{GOOD}listserver - Disconnected.\nlistserver - Connected to 127.0.0.1:14900.\n"
            ),
            true,
        );
        assert!(matches!(l.listserver, Light::Ok(_)));
        let l = Lights::read("listserver - [Error] Bad server password\n", true);
        assert_eq!(l.listserver, Light::Bad("Bad server password".into()));
    }
}

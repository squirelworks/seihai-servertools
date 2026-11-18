//! NPC-server database objects over an NPC Control (NC) login: database
//! NPCs, classes and weapons. Blocking calls for the editor page; each one
//! logs in, does its work and closes, so no session outlives a click.
//!
//! The NC client is seihai-rc's (`seihai_rc::nc`). A save sends the set and
//! then a get for the same target, and succeeds only when that read shows
//! the text this client can represent for what was sent
//! (`seihai_rc::nc::observe_script`): an answer such as a chat line, a
//! class name or a weapon list is not proof the script is there.

use std::time::{Duration, Instant};

use seihai_rc::nc::{
    class_delete_message, class_get_message, class_set_message, npc_reset_message,
    npc_script_get_message, npc_script_set_message, observe_script, representable_npc_class_script,
    representable_weapon_script, weapon_delete_message, weapon_get_message, weapon_list_message,
    weapon_set_message, NcEvent, NcSession, ScriptSubject,
};

pub use seihai_rc::nc::NcLists;

/// Where and as whom to log in. The password is sent only to `addr`.
#[derive(Clone, Debug, Default)]
pub struct Login {
    pub addr: String,
    pub account: String,
    pub password: String,
    /// How long a login or one read may take.
    pub timeout: Duration,
}

/// One database object to read or write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    Npc(u32),
    Class(String),
    Weapon(String),
}

impl Target {
    fn subject(&self) -> ScriptSubject {
        match self {
            Target::Npc(id) => ScriptSubject::Npc(*id),
            Target::Class(n) => ScriptSubject::Class(n.clone()),
            Target::Weapon(n) => ScriptSubject::Weapon(n.clone()),
        }
    }

    fn get(&self) -> Result<Vec<u8>, String> {
        match self {
            Target::Npc(id) => npc_script_get_message(*id),
            Target::Class(n) => class_get_message(n),
            Target::Weapon(n) => weapon_get_message(n),
        }
        .map_err(|e| e.to_string())
    }
}

/// A script read: the text, and for a weapon its image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Script {
    pub text: String,
    pub image: Option<String>,
}

impl Login {
    fn timeout(&self) -> Duration {
        if self.timeout.is_zero() {
            Duration::from_secs(10)
        } else {
            self.timeout
        }
    }

    fn open(&self) -> Result<NcSession, String> {
        let addr = self.addr.trim();
        if addr.is_empty() {
            return Err("Type the NC address (host:port).".into());
        }
        if self.account.trim().is_empty() {
            return Err("Type the NC account.".into());
        }
        NcSession::login(addr, self.account.trim(), &self.password, self.timeout())
            .map_err(|e| format!("NC login: {e}"))
    }
}

/// How long to wait for more of a list once it stops arriving.
const SETTLE: Duration = Duration::from_millis(400);

/// The NPC list, class names and weapon names the server sends an NC.
pub fn list(login: &Login) -> Result<NcLists, String> {
    let mut s = login.open()?;
    s.send(&[weapon_list_message()])
        .map_err(|e| e.to_string())?;
    let deadline = Instant::now() + login.timeout();
    // Wait for the weapon list, then for anything still coming.
    let mut got_weapons = false;
    while !got_weapons {
        match s.next_event(deadline).map_err(|e| e.to_string())? {
            Some(e) => {
                // The session folds each event into its lists itself.
                got_weapons = matches!(e, NcEvent::Weapons(_));
                if let NcEvent::Disconnect(why) = e {
                    return Err(format!("disconnected: {why}"));
                }
            }
            None => break,
        }
    }
    s.settle(SETTLE, deadline).map_err(|e| e.to_string())?;
    let lists = s.lists.clone();
    s.close();
    Ok(lists)
}

/// Read one script.
pub fn read(login: &Login, target: &Target) -> Result<Script, String> {
    let mut s = login.open()?;
    s.send(&[target.get()?]).map_err(|e| e.to_string())?;
    let subject = target.subject();
    let deadline = Instant::now() + login.timeout();
    loop {
        let Some(e) = s.next_event(deadline).map_err(|e| e.to_string())? else {
            return Err(format!(
                "no {} arrived",
                seihai_rc::nc::script_subject_phrase(&subject)
            ));
        };
        if let Some(text) = seihai_rc::nc::matching_script(&subject, &e) {
            let image = match &e {
                NcEvent::Weapon(w) => Some(w.image.clone()),
                _ => None,
            };
            let out = Script {
                text: text.to_string(),
                image,
            };
            s.close();
            return Ok(out);
        }
        if let NcEvent::Disconnect(why) = e {
            return Err(format!("disconnected: {why}"));
        }
        if let Some(why) = seihai_rc::nc::rc_chat_refusal(&e) {
            return Err(why);
        }
    }
}

/// Save a script and read it back. `image` is a weapon's image (ignored
/// for NPCs and classes). `Ok` is the line to show.
pub fn save(login: &Login, target: &Target, text: &str, image: &str) -> Result<String, String> {
    let set = match target {
        Target::Npc(id) => npc_script_set_message(*id, text),
        Target::Class(n) => class_set_message(n, text),
        Target::Weapon(n) => weapon_set_message(n, image, text),
    }
    .map_err(|e| e.to_string())?;
    let expected = match target {
        Target::Weapon(_) => representable_weapon_script(text),
        _ => representable_npc_class_script(text),
    };
    let mut s = login.open()?;
    s.send(&[set, target.get()?]).map_err(|e| e.to_string())?;
    let subject = target.subject();
    let deadline = Instant::now() + login.timeout();
    let mut events = Vec::new();
    loop {
        let obs = observe_script(&subject, &expected, &events);
        if obs != seihai_rc::nc::ScriptObservation::NoMatchingReply {
            s.close();
            return obs.outcome_line(&subject, expected.len());
        }
        match s.next_event(deadline).map_err(|e| e.to_string())? {
            Some(e) => events.push(e),
            None => {
                s.close();
                return obs.outcome_line(&subject, expected.len());
            }
        }
    }
}

/// Delete a class or a weapon, or reset an NPC (NPC Control has no other
/// action on an NPC's whole object here). `Ok` is the line to show.
pub fn remove_or_reset(login: &Login, target: &Target) -> Result<String, String> {
    let m = match target {
        Target::Npc(id) => npc_reset_message(*id),
        Target::Class(n) => class_delete_message(n),
        Target::Weapon(n) => weapon_delete_message(n),
    }
    .map_err(|e| e.to_string())?;
    let mut s = login.open()?;
    s.send(&[m]).map_err(|e| e.to_string())?;
    let deadline = Instant::now() + login.timeout();
    let events = s.settle(SETTLE, deadline).map_err(|e| e.to_string())?;
    s.close();
    if let Some(why) = events.iter().find_map(seihai_rc::nc::rc_chat_refusal) {
        return Err(why);
    }
    let done = events.iter().any(|e| match (target, e) {
        (Target::Npc(id), NcEvent::NpcReset(got)) => got == id,
        (Target::Class(n), NcEvent::ClassDeleted(got)) => got == n,
        (Target::Weapon(n), NcEvent::Weapons(names)) => !names.contains(n),
        _ => false,
    });
    match (target, done) {
        (Target::Npc(id), true) => Ok(format!("NPC {id} reset.")),
        (Target::Class(n), true) => Ok(format!("Class {n} deleted.")),
        (Target::Weapon(n), true) => Ok(format!("Weapon {n} deleted.")),
        (Target::Npc(id), false) => Err(format!(
            "no reset came back for NPC {id}; this server may not reset NPCs yet"
        )),
        (Target::Class(n), false) => Err(format!("class {n}: no delete came back")),
        (Target::Weapon(n), false) => Err(format!("weapon {n} is still on the list")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// One NC message as the server writes it.
    fn msg(id: u8, payload: &[u8]) -> Vec<u8> {
        let mut m = vec![id + 0x20];
        m.extend_from_slice(payload);
        m.push(b'\n');
        m
    }

    fn gstring(s: &[u8]) -> Vec<u8> {
        let mut v = vec![s.len() as u8 + 0x20];
        v.extend_from_slice(s);
        v
    }

    fn write_frame(out: &mut impl Write, body: &[u8]) {
        let z = seihai_core::protocol::zlib_compress(body).unwrap();
        out.write_all(&(z.len() as u16).to_be_bytes()).unwrap();
        out.write_all(&z).unwrap();
    }

    fn read_frame(input: &mut impl Read) -> Option<Vec<u8>> {
        let mut len = [0u8; 2];
        input.read_exact(&mut len).ok()?;
        let mut z = vec![0; u16::from_be_bytes(len) as usize];
        input.read_exact(&mut z).ok()?;
        seihai_core::protocol::zlib_decompress(&z).ok()
    }

    /// A one-class, one-weapon NC server that keeps what it is sent, in
    /// the layouts seihai-rc reads (its `docs/PROTOCOL.md`).
    fn mock() -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = l.local_addr().unwrap().to_string();
        std::thread::spawn(move || {
            let mut class = b"\"if (created) {\",\"  this.a = 1;\",}".to_vec();
            let mut weapon = b"if (weaponfired) {\xa7  say2 hi;\xa7}\xa7".to_vec();
            let mut have_weapon = true;
            for stream in l.incoming() {
                let Ok(mut s) = stream else { return };
                if read_frame(&mut s).is_none() {
                    continue;
                }
                let mut first = msg(163, b"Lamp");
                first.extend(msg(158, b"  !R$bellS T(start.nw"));
                write_frame(&mut s, &first);
                while let Some(body) = read_frame(&mut s) {
                    let mut out = Vec::new();
                    for line in body.split(|&b| b == b'\n').filter(|l| !l.is_empty()) {
                        let (id, rest) = (line[0] - 0x20, &line[1..]);
                        match id {
                            115 => {
                                let mut p = Vec::new();
                                if have_weapon {
                                    p.extend(gstring(b"Bow"));
                                }
                                out.extend(msg(167, &p));
                            }
                            112 => {
                                let mut p = gstring(rest);
                                p.extend_from_slice(&class);
                                out.extend(msg(162, &p));
                            }
                            113 => {
                                let n = (rest[0] - 0x20) as usize;
                                class = rest[1 + n..].to_vec();
                                out.extend(msg(163, &rest[1..1 + n]));
                            }
                            116 => {
                                let mut p = gstring(rest);
                                p.push(0x20);
                                p.extend(gstring(b"bow.png"));
                                p.push(0x21);
                                p.push(((weapon.len() >> 7) + 0x20) as u8);
                                p.push(((weapon.len() & 0x7f) + 0x20) as u8);
                                p.extend_from_slice(&weapon);
                                out.extend(msg(33, &p));
                            }
                            117 => {
                                let n = (rest[0] - 0x20) as usize;
                                let r = &rest[1 + n..];
                                let m = (r[0] - 0x20) as usize;
                                weapon = r[1 + m..].to_vec();
                                out.extend(msg(74, b"Weapon Bow updated"));
                            }
                            118 => {
                                have_weapon = false;
                                out.extend(msg(167, b""));
                            }
                            105 => out.extend(msg(74, b"error: no NPC reset here")),
                            _ => {}
                        }
                    }
                    if !out.is_empty() {
                        write_frame(&mut s, &out);
                    }
                }
            }
        });
        addr
    }

    fn login(addr: &str) -> Login {
        Login {
            addr: addr.into(),
            account: "boss".into(),
            password: "pw".into(),
            timeout: Duration::from_secs(5),
        }
    }

    #[test]
    fn lists_reads_saves_and_deletes() {
        let l = login(&mock());
        let lists = list(&l).unwrap();
        assert_eq!(lists.classes, ["Lamp"]);
        assert_eq!(lists.weapons, ["Bow"]);
        assert_eq!(lists.npcs.len(), 1);

        let c = read(&l, &Target::Class("Lamp".into())).unwrap();
        assert_eq!(c.text, "if (created) {\n  this.a = 1;\n}");
        assert_eq!(c.image, None);
        let line = save(
            &l,
            &Target::Class("Lamp".into()),
            "if (created) {\n  this.b = 2;\n}\n",
            "",
        )
        .unwrap();
        assert_eq!(line, "observed class Lamp");
        assert!(read(&l, &Target::Class("Lamp".into()))
            .unwrap()
            .text
            .contains("this.b"));

        let w = read(&l, &Target::Weapon("Bow".into())).unwrap();
        assert_eq!(w.image.as_deref(), Some("bow.png"));
        assert!(w.text.contains("say2 hi;"), "{:?}", w.text);
        save(
            &l,
            &Target::Weapon("Bow".into()),
            "if (weaponfired) {\n  say2 yo;\n}\n",
            "bow.png",
        )
        .unwrap();
        assert!(read(&l, &Target::Weapon("Bow".into()))
            .unwrap()
            .text
            .contains("say2 yo;"));
        assert_eq!(
            remove_or_reset(&l, &Target::Weapon("Bow".into())).unwrap(),
            "Weapon Bow deleted."
        );
        let e = remove_or_reset(&l, &Target::Npc(1)).unwrap_err();
        assert!(e.contains("error: no NPC reset here"), "{e}");
    }

    #[test]
    fn an_empty_form_is_refused_before_connecting() {
        let e = list(&Login::default()).unwrap_err();
        assert!(e.contains("address"), "{e}");
    }
}

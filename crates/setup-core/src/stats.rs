//! Leaderboard pages from a seihai-gserver world's accounts: the job of the
//! 1999 `gserverstat` / `gserverstat2` tools and the later GServer Stats
//! (behaviour notes: Graal_analysis `tools/classic-tools/gserverstat.md`).
//!
//! Reads `accounts/<account>.txt` (seihai-gserver's `KEY value` lines),
//! ranks by one field, and writes a small HTML table: place, nickname,
//! value, at most 50 rows. Like the originals it never shows a password or
//! an account name, and it skips load-only accounts. Read only: no account
//! file is changed.

use std::path::Path;

/// One ranking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mode {
    /// The name on the command line and in the file name (`kills.html`).
    pub name: &'static str,
    /// The column title.
    pub title: &'static str,
    /// The account key read, and which number on its line (`SWORD 2 x.png`).
    key: &'static str,
    /// Highest first, or lowest first (`lowap`).
    descending: bool,
}

/// The modes gserverstat had that seihai-gserver's accounts can fill.
/// `bombpower` and `onlinetime` are not stored by seihai-gserver.
pub const MODES: [Mode; 12] = [
    Mode {
        name: "kills",
        title: "Kills",
        key: "KILLS",
        descending: true,
    },
    Mode {
        name: "deaths",
        title: "Deaths",
        key: "DEATHS",
        descending: true,
    },
    Mode {
        name: "rupees",
        title: "Rupees",
        key: "RUPEES",
        descending: true,
    },
    Mode {
        name: "highap",
        title: "Alignment",
        key: "ALIGN",
        descending: true,
    },
    Mode {
        name: "lowap",
        title: "Alignment",
        key: "ALIGN",
        descending: false,
    },
    Mode {
        name: "maxpower",
        title: "Max hearts",
        key: "MAXHP",
        descending: true,
    },
    Mode {
        name: "power",
        title: "Hearts",
        key: "HP",
        descending: true,
    },
    Mode {
        name: "arrows",
        title: "Arrows",
        key: "ARROWS",
        descending: true,
    },
    Mode {
        name: "bombs",
        title: "Bombs",
        key: "BOMBS",
        descending: true,
    },
    Mode {
        name: "glovepower",
        title: "Glove",
        key: "GLOVE",
        descending: true,
    },
    Mode {
        name: "swordpower",
        title: "Sword",
        key: "SWORD",
        descending: true,
    },
    Mode {
        name: "shieldpower",
        title: "Shield",
        key: "SHIELD",
        descending: true,
    },
];

/// The mode called `name`, any case.
pub fn mode(name: &str) -> Option<Mode> {
    MODES
        .iter()
        .copied()
        .find(|m| m.name.eq_ignore_ascii_case(name))
}

/// The fields of one account that a page may show.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub nick: String,
    values: Vec<(String, f64)>,
    kills: f64,
}

impl Entry {
    fn value(&self, key: &str) -> f64 {
        self.values
            .iter()
            .find(|(k, _)| k == key)
            .map_or(0.0, |(_, v)| *v)
    }
}

/// Read one account file. `None` for a load-only account or one that is
/// not a seihai-gserver account.
pub fn read_account(text: &str) -> Option<Entry> {
    let mut lines = text.lines().map(|l| l.trim_end_matches('\r'));
    if !lines.next()?.trim().starts_with("SEIHAIACC") {
        return None;
    }
    let mut e = Entry {
        nick: String::new(),
        values: Vec::new(),
        kills: 0.0,
    };
    for line in lines {
        let (key, value) = line.split_once(' ').unwrap_or((line, ""));
        match key {
            "NICK" => e.nick = value.trim().to_string(),
            "LOADONLY" if !matches!(value.trim(), "0" | "false" | "no") => return None,
            k if MODES.iter().any(|m| m.key == k) => {
                let n = value.split_whitespace().next().and_then(|v| v.parse().ok());
                if let Some(n) = n {
                    e.values.push((k.to_string(), n));
                    if k == "KILLS" {
                        e.kills = n;
                    }
                }
            }
            _ => {}
        }
    }
    Some(e)
}

/// Every account under `accounts`, in file name order.
pub fn read_accounts(accounts: &Path) -> Result<Vec<Entry>, String> {
    let mut files: Vec<_> = std::fs::read_dir(accounts)
        .map_err(|e| format!("{}: {e}", accounts.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("txt")))
        .collect();
    files.sort();
    Ok(files
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .filter_map(|t| read_account(&t))
        .collect())
}

/// The top 50 for `mode`: `(nickname, value)`. Alignment ties keep fewer
/// kills first for `highap` and more kills first for `lowap`, as the
/// originals did; other ties keep file order.
pub fn rank(entries: &[Entry], mode: Mode) -> Vec<(String, f64)> {
    let mut v: Vec<&Entry> = entries.iter().collect();
    v.sort_by(|a, b| {
        let (x, y) = (a.value(mode.key), b.value(mode.key));
        let main = if mode.descending {
            y.total_cmp(&x)
        } else {
            x.total_cmp(&y)
        };
        let tie = if mode.key != "ALIGN" {
            std::cmp::Ordering::Equal
        } else if mode.descending {
            a.kills.total_cmp(&b.kills)
        } else {
            b.kills.total_cmp(&a.kills)
        };
        main.then(tie)
    });
    v.into_iter()
        .take(50)
        .map(|e| (e.nick.clone(), e.value(mode.key)))
        .collect()
}

fn escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            c => c.to_string(),
        })
        .collect()
}

/// The HTML page for one ranking.
pub fn page(mode: Mode, rows: &[(String, f64)]) -> String {
    let mut out = format!(
        "<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"><title>{t}</title></head>\n<body>\n<table>\n<tr><th>Place</th><th>Name</th><th>{t}</th></tr>\n",
        t = escape(mode.title)
    );
    for (i, (nick, value)) in rows.iter().enumerate() {
        let value = if value.fract() == 0.0 {
            format!("{}", *value as i64)
        } else {
            value.to_string()
        };
        out.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            i + 1,
            escape(nick),
            value
        ));
    }
    out.push_str("</table>\n</body></html>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acc(nick: &str, extra: &str) -> String {
        format!("SEIHAIACC 1\nPASSWORD pbkdf2-sha256:1:aa:bb\nNICK {nick}\n{extra}")
    }

    #[test]
    fn reads_fields_and_skips_load_only_and_foreign_files() {
        let e = read_account(&acc("Squirrel", "KILLS 4\nSWORD 2 sword2.png\nALIGN 70\n")).unwrap();
        assert_eq!(e.nick, "Squirrel");
        assert_eq!(e.value("SWORD"), 2.0);
        assert_eq!(e.value("RUPEES"), 0.0);
        assert!(read_account(&acc("x", "LOADONLY 1\n")).is_none());
        assert!(read_account(&acc("x", "LOADONLY 0\n")).is_some());
        assert!(read_account("GRACC001\nNICK x\n").is_none());
    }

    #[test]
    fn ranks_with_alignment_ties_and_escapes_html() {
        let entries: Vec<Entry> = [
            acc("<b>a</b>", "ALIGN 50\nKILLS 3\n"),
            acc("b", "ALIGN 50\nKILLS 1\n"),
            acc("c", "ALIGN 90\nKILLS 9\n"),
        ]
        .iter()
        .filter_map(|t| read_account(t))
        .collect();
        let high = rank(&entries, mode("HighAP").unwrap());
        let names: Vec<&str> = high.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["c", "b", "<b>a</b>"]);
        let low = rank(&entries, mode("lowap").unwrap());
        let names: Vec<&str> = low.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["<b>a</b>", "b", "c"]);
        let html = page(mode("highap").unwrap(), &high);
        assert!(html.contains("&lt;b&gt;a&lt;/b&gt;"));
        assert!(!html.contains("pbkdf2"));
        assert!(html.contains("<td>1</td><td>c</td><td>90</td>"));
    }

    #[test]
    fn at_most_fifty_rows() {
        let entries: Vec<Entry> = (0..60)
            .filter_map(|i| read_account(&acc(&format!("p{i}"), &format!("KILLS {i}\n"))))
            .collect();
        let rows = rank(&entries, mode("kills").unwrap());
        assert_eq!(rows.len(), 50);
        assert_eq!(rows[0], ("p59".to_string(), 59.0));
    }
}

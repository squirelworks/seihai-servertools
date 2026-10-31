//! `serveroptions.txt`, the gserver fork's settings file
//! (`servers/<name>/config/serveroptions.txt`).
//!
//! The format, as a fact: one `key = value` per line, `#` starts a comment
//! line, keys are compared without case, blank lines and unknown keys are
//! allowed. The file is the owner's: [`OptionsTxt`] keeps every line exactly
//! as read so a writer can change one value and leave the rest alone.
//!
//! [`OptionsTxt::set`] changes one value in place (or appends the key) and
//! puts a `# Changed by Seihai server setup, YYYY-MM-DD` line above it.
//! Saving goes through [`super::backup::save`], which keeps a dated backup.

use std::fs;
use std::io;
use std::path::Path;

/// The start of the comment line put above every changed key.
pub const CHANGE_COMMENT: &str = "# Changed by Seihai server setup";

/// One line of the file, as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// `key = value`. `raw` is the whole line without its line ending.
    Setting {
        key: String,
        value: String,
        raw: String,
    },
    /// A comment, a blank line, or a line with no `=`: kept verbatim.
    Other(String),
}

/// A parsed `serveroptions.txt` that remembers every line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionsTxt {
    pub lines: Vec<Line>,
    /// `"\r\n"` when the file used CRLF line endings, else `"\n"`.
    pub newline: &'static str,
    /// True when the last line ended with a line ending.
    pub trailing_newline: bool,
}

impl Default for OptionsTxt {
    fn default() -> Self {
        OptionsTxt::parse("")
    }
}

impl OptionsTxt {
    /// Parse the text of a `serveroptions.txt`.
    pub fn parse(text: &str) -> Self {
        let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let trailing_newline = text.ends_with('\n');
        let lines = text
            .lines()
            .map(|l| {
                let raw = l.trim_end_matches('\r').to_string();
                let trimmed = raw.trim_start();
                if trimmed.starts_with('#') || trimmed.starts_with("//") {
                    return Line::Other(raw);
                }
                match raw.split_once('=') {
                    Some((k, v)) if !k.trim().is_empty() => Line::Setting {
                        key: k.trim().to_string(),
                        value: v.trim().to_string(),
                        raw,
                    },
                    _ => Line::Other(raw),
                }
            })
            .collect();
        OptionsTxt {
            lines,
            newline,
            trailing_newline,
        }
    }

    /// Read and parse a file. The bytes are read as Latin-1-safe lossy UTF-8.
    pub fn load(path: &Path) -> io::Result<Self> {
        let data = fs::read(path)?;
        Ok(Self::parse(&String::from_utf8_lossy(&data)))
    }

    /// The value of `key` (case-insensitive). With the key set twice, the
    /// last one wins, as for the gserver.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.lines.iter().rev().find_map(|l| match l {
            Line::Setting { key: k, value, .. } if k.eq_ignore_ascii_case(key) => {
                Some(value.as_str())
            }
            _ => None,
        })
    }

    /// Set `key` to `value`. The line that counts (the last one with that
    /// key) is changed in place, keeping the key's spelling and the spacing
    /// around `=`; a missing key is added at the end. A change comment dated
    /// `date` (`YYYY-MM-DD`) goes on the line above, or replaces the one
    /// already there. Returns false, and changes nothing, when the value is
    /// already `value`.
    pub fn set(&mut self, key: &str, value: &str, date: &str) -> bool {
        let comment = format!("{CHANGE_COMMENT}, {date}");
        let found = self.lines.iter().rposition(
            |l| matches!(l, Line::Setting { key: k, .. } if k.eq_ignore_ascii_case(key)),
        );
        match found {
            Some(i) => {
                let Line::Setting {
                    key: k,
                    value: v,
                    raw,
                } = &self.lines[i]
                else {
                    unreachable!("rposition found a setting");
                };
                if v == value {
                    return false;
                }
                let eq = raw.find('=').unwrap_or(raw.len());
                let after = &raw[eq + 1..];
                let pad = &after[..after.len() - after.trim_start().len()];
                let raw = format!("{}={}{}", &raw[..eq], pad, value);
                self.lines[i] = Line::Setting {
                    key: k.clone(),
                    value: value.to_string(),
                    raw,
                };
                let above = i.checked_sub(1).map(|j| &self.lines[j]);
                match above {
                    Some(Line::Other(c)) if c.starts_with(CHANGE_COMMENT) => {
                        self.lines[i - 1] = Line::Other(comment);
                    }
                    _ => self.lines.insert(i, Line::Other(comment)),
                }
            }
            None => {
                // The old last line now needs an ending; so does the new one.
                self.trailing_newline = true;
                self.lines.push(Line::Other(comment));
                self.lines.push(Line::Setting {
                    key: key.to_string(),
                    value: value.to_string(),
                    raw: format!("{key} = {value}"),
                });
            }
        }
        true
    }

    /// The keys whose values differ between `self` (before) and `after`,
    /// as `(key, before, after)`, in the order of `after`, then keys only
    /// `self` had. For the wizard's review step.
    pub fn diff(&self, after: &OptionsTxt) -> Vec<(String, Option<String>, Option<String>)> {
        let mut keys: Vec<String> = Vec::new();
        for l in after.lines.iter().chain(&self.lines) {
            if let Line::Setting { key, .. } = l {
                if !keys.iter().any(|k| k.eq_ignore_ascii_case(key)) {
                    keys.push(key.clone());
                }
            }
        }
        keys.into_iter()
            .filter_map(|k| {
                let a = self.get(&k).map(str::to_string);
                let b = after.get(&k).map(str::to_string);
                (a != b).then_some((k, a, b))
            })
            .collect()
    }

    /// The file's text again. With no change it is the same text as read.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for (i, line) in self.lines.iter().enumerate() {
            if i > 0 {
                out.push_str(self.newline);
            }
            match line {
                Line::Setting { raw, .. } | Line::Other(raw) => out.push_str(raw),
            }
        }
        if self.trailing_newline && !self.lines.is_empty() {
            out.push_str(self.newline);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# Server options\nname = My Server\nserverinterface = 127.0.0.1\n\nserverport=14802\n  # indented comment\nmystery key = kept as is\nName = Second\n";

    #[test]
    fn reads_values_case_insensitively_last_wins() {
        let o = OptionsTxt::parse(SAMPLE);
        assert_eq!(o.get("serverinterface"), Some("127.0.0.1"));
        assert_eq!(o.get("SERVERPORT"), Some("14802"));
        assert_eq!(o.get("name"), Some("Second"));
        assert_eq!(o.get("missing"), None);
    }

    #[test]
    fn round_trips_lf_and_crlf_byte_for_byte() {
        for text in [
            SAMPLE.to_string(),
            SAMPLE.replace('\n', "\r\n"),
            SAMPLE.trim_end().to_string(),
            String::new(),
        ] {
            assert_eq!(OptionsTxt::parse(&text).render(), text);
        }
    }

    #[test]
    fn set_changes_in_place_and_keeps_everything_else() {
        let mut o = OptionsTxt::parse(SAMPLE);
        assert!(o.set("SERVERINTERFACE", "AUTO", "2026-10-31"));
        let text = o.render();
        assert_eq!(
            text,
            SAMPLE.replace(
                "serverinterface = 127.0.0.1\n",
                "# Changed by Seihai server setup, 2026-10-31\nserverinterface = AUTO\n"
            )
        );
        assert_eq!(o.get("serverinterface"), Some("AUTO"));
    }

    #[test]
    fn set_keeps_tight_spacing_and_crlf() {
        let mut o = OptionsTxt::parse("serverport=14802\r\nupnp = false\r\n");
        o.set("serverport", "14900", "2026-10-31");
        assert_eq!(
            o.render(),
            "# Changed by Seihai server setup, 2026-10-31\r\nserverport=14900\r\nupnp = false\r\n"
        );
    }

    #[test]
    fn set_same_value_changes_nothing() {
        let mut o = OptionsTxt::parse(SAMPLE);
        assert!(!o.set("serverport", "14802", "2026-10-31"));
        assert_eq!(o.render(), SAMPLE);
    }

    #[test]
    fn set_twice_keeps_one_comment_with_the_new_date() {
        let mut o = OptionsTxt::parse(SAMPLE);
        o.set("upnp", "true", "2026-10-30");
        o.set("upnp", "false", "2026-10-31");
        let text = o.render();
        assert_eq!(text.matches(CHANGE_COMMENT).count(), 1, "{text}");
        assert!(text.ends_with(
            "Name = Second\n# Changed by Seihai server setup, 2026-10-31\nupnp = false\n"
        ));
    }

    #[test]
    fn set_changes_the_last_of_duplicate_keys() {
        let mut o = OptionsTxt::parse(SAMPLE);
        o.set("name", "Lab", "2026-10-31");
        assert!(o.render().contains("name = My Server\n"));
        assert!(o.render().contains("Name = Lab\n"));
    }

    #[test]
    fn set_appends_to_a_file_without_final_newline() {
        let mut o = OptionsTxt::parse("name = x");
        o.set("localhost", "false", "2026-10-31");
        assert_eq!(
            o.render(),
            "name = x\n# Changed by Seihai server setup, 2026-10-31\nlocalhost = false\n"
        );
        let mut empty = OptionsTxt::parse("");
        empty.set("a", "b", "2026-10-31");
        assert_eq!(
            empty.render(),
            "# Changed by Seihai server setup, 2026-10-31\na = b\n"
        );
    }

    #[test]
    fn diff_lists_changed_added_and_removed_keys() {
        let before = OptionsTxt::parse("a = 1\nb = 2\nc = 3\n");
        let after = OptionsTxt::parse("a = 1\nb = 9\nd = 4\n");
        assert_eq!(
            before.diff(&after),
            vec![
                ("b".into(), Some("2".into()), Some("9".into())),
                ("d".into(), None, Some("4".into())),
                ("c".into(), Some("3".into()), None),
            ]
        );
    }

    #[test]
    fn comments_with_equals_are_not_settings() {
        let o = OptionsTxt::parse("# a = b\n");
        assert_eq!(o.get("# a"), None);
        assert!(matches!(o.lines[0], Line::Other(_)));
    }
}

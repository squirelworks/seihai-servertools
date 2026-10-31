//! `serveroptions.txt`, the gserver fork's settings file
//! (`servers/<name>/config/serveroptions.txt`).
//!
//! The format, as a fact: one `key = value` per line, `#` starts a comment
//! line, keys are compared without case, blank lines and unknown keys are
//! allowed. The file is the owner's: [`OptionsTxt`] keeps every line exactly
//! as read so a writer can change one value and leave the rest alone.
//!
//! **Today:** reading. **Setup step 1** (`docs/NEXT.md`) adds `set`, `remove`,
//! the change-comment line and saving through [`super::backup`].

use std::fs;
use std::io;
use std::path::Path;

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
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OptionsTxt {
    pub lines: Vec<Line>,
    /// `"\r\n"` when the file used CRLF line endings, else `"\n"`.
    pub newline: &'static str,
    /// True when the last line ended with a line ending.
    pub trailing_newline: bool,
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
    fn comments_with_equals_are_not_settings() {
        let o = OptionsTxt::parse("# a = b\n");
        assert_eq!(o.get("# a"), None);
        assert!(matches!(o.lines[0], Line::Other(_)));
    }
}

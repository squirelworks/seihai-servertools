//! A line diff for the editors' review before an upload.
//!
//! The editors keep the bytes they downloaded. Before an upload they show
//! what changed against those bytes, and when the server's copy moved in
//! between they show what the server has now. Both are this diff.
//!
//! Lines are compared as bytes and shown lossily as UTF-8. The common
//! start and end are trimmed first; the middle is a longest-common-
//! subsequence diff. A middle too big for that is shown as all removed then
//! all added, so a huge file can never stall the window.

/// One line of a diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    Same(String),
    Removed(String),
    Added(String),
}

/// The most cells (old middle lines times new middle lines) the LCS table
/// may hold.
const MAX_CELLS: usize = 4_000_000;

fn split(data: &[u8]) -> Vec<&[u8]> {
    if data.is_empty() {
        return Vec::new();
    }
    let body = data.strip_suffix(b"\n").unwrap_or(data);
    body.split(|&b| b == b'\n')
        .map(|l| l.strip_suffix(b"\r").unwrap_or(l))
        .collect()
}

fn text(line: &[u8]) -> String {
    String::from_utf8_lossy(line).into_owned()
}

/// Every line of `old` and `new`, marked same, removed or added.
pub fn lines(old: &[u8], new: &[u8]) -> Vec<Line> {
    let a = split(old);
    let b = split(new);
    let mut head = 0;
    while head < a.len() && head < b.len() && a[head] == b[head] {
        head += 1;
    }
    let mut tail = 0;
    while tail < a.len() - head
        && tail < b.len() - head
        && a[a.len() - 1 - tail] == b[b.len() - 1 - tail]
    {
        tail += 1;
    }
    let am = &a[head..a.len() - tail];
    let bm = &b[head..b.len() - tail];

    let mut out: Vec<Line> = a[..head].iter().map(|l| Line::Same(text(l))).collect();
    if am.len().saturating_mul(bm.len()) > MAX_CELLS {
        out.extend(am.iter().map(|l| Line::Removed(text(l))));
        out.extend(bm.iter().map(|l| Line::Added(text(l))));
    } else {
        // lcs[i][j]: the longest common run of am[i..] and bm[j..].
        let w = bm.len() + 1;
        let mut lcs = vec![0u32; (am.len() + 1) * w];
        for i in (0..am.len()).rev() {
            for j in (0..bm.len()).rev() {
                lcs[i * w + j] = if am[i] == bm[j] {
                    lcs[(i + 1) * w + j + 1] + 1
                } else {
                    lcs[(i + 1) * w + j].max(lcs[i * w + j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < am.len() && j < bm.len() {
            if am[i] == bm[j] {
                out.push(Line::Same(text(am[i])));
                i += 1;
                j += 1;
            } else if lcs[(i + 1) * w + j] >= lcs[i * w + j + 1] {
                out.push(Line::Removed(text(am[i])));
                i += 1;
            } else {
                out.push(Line::Added(text(bm[j])));
                j += 1;
            }
        }
        out.extend(am[i..].iter().map(|l| Line::Removed(text(l))));
        out.extend(bm[j..].iter().map(|l| Line::Added(text(l))));
    }
    out.extend(a[a.len() - tail..].iter().map(|l| Line::Same(text(l))));
    out
}

/// How many lines were removed and added.
pub fn counts(diff: &[Line]) -> (usize, usize) {
    diff.iter().fold((0, 0), |(r, a), l| match l {
        Line::Removed(_) => (r + 1, a),
        Line::Added(_) => (r, a + 1),
        Line::Same(_) => (r, a),
    })
}

/// The changed lines with `context` unchanged lines around each change,
/// `-` for removed and `+` for added. Skipped runs show as `...`.
/// Empty when nothing changed.
pub fn render(diff: &[Line], context: usize) -> String {
    let changed: Vec<usize> = diff
        .iter()
        .enumerate()
        .filter(|(_, l)| !matches!(l, Line::Same(_)))
        .map(|(i, _)| i)
        .collect();
    if changed.is_empty() {
        return String::new();
    }
    let keep = |i: usize| {
        changed
            .iter()
            .any(|&c| i + context >= c && i <= c + context)
    };
    let mut out = String::new();
    let mut skipped = false;
    for (i, line) in diff.iter().enumerate() {
        if !keep(i) {
            skipped = true;
            continue;
        }
        if skipped {
            out.push_str("...\n");
            skipped = false;
        }
        let (mark, t) = match line {
            Line::Same(t) => (' ', t),
            Line::Removed(t) => ('-', t),
            Line::Added(t) => ('+', t),
        };
        out.push(mark);
        out.push(' ');
        out.push_str(t);
        out.push('\n');
    }
    if skipped {
        out.push_str("...\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_bytes_have_no_changes() {
        let d = lines(b"a\nb\n", b"a\nb\n");
        assert_eq!(counts(&d), (0, 0));
        assert_eq!(render(&d, 2), "");
    }

    #[test]
    fn one_changed_line() {
        let old = b"GLEVNW01\nBOARD 0 0 1 0 AA\nNPC - 1 1\n";
        let new = b"GLEVNW01\nBOARD 0 0 1 0 AB\nNPC - 1 1\n";
        let d = lines(old, new);
        assert_eq!(counts(&d), (1, 1));
        assert_eq!(
            render(&d, 0),
            "...\n- BOARD 0 0 1 0 AA\n+ BOARD 0 0 1 0 AB\n...\n"
        );
    }

    #[test]
    fn inserted_and_removed_lines_keep_order() {
        let d = lines(b"a\nb\nc\nd\n", b"a\nc\nx\nd\n");
        assert_eq!(
            d,
            vec![
                Line::Same("a".into()),
                Line::Removed("b".into()),
                Line::Same("c".into()),
                Line::Added("x".into()),
                Line::Same("d".into()),
            ]
        );
    }

    #[test]
    fn crlf_and_missing_last_newline_are_the_same_lines() {
        assert_eq!(counts(&lines(b"a\r\nb\r\n", b"a\nb")), (0, 0));
    }

    #[test]
    fn empty_sides() {
        assert_eq!(counts(&lines(b"", b"a\nb\n")), (0, 2));
        assert_eq!(counts(&lines(b"a\n", b"")), (1, 0));
    }

    #[test]
    fn huge_middle_falls_back_without_a_table() {
        let old: Vec<u8> = (0..3000)
            .flat_map(|i| format!("o{i}\n").into_bytes())
            .collect();
        let new: Vec<u8> = (0..3000)
            .flat_map(|i| format!("n{i}\n").into_bytes())
            .collect();
        assert_eq!(counts(&lines(&old, &new)), (3000, 3000));
    }
}

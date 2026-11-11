//! `seihai-suite-stress probe`: one scripted client against a gserver you
//! started yourself. It logs in with one version profile, runs a steps file
//! and prints every message the server sends, with a timestamp. Moved here
//! from seihai-gserver-v2's `tools/lab/npcprobe`, where it drove the
//! 2026-10-31 NPC-server lab (`docs/lab/probe/`).

/// One line of a steps file.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// `wait SECONDS`: only read from the server.
    Wait(f32),
    /// `walk X Y [DIR]`: send a position (DIR defaults to 2, down).
    Walk { x: f32, y: f32, dir: u8 },
    /// `chat TEXT`.
    Chat(String),
    /// `trigger X Y ACTION [PARAMS]`.
    Trigger {
        x: f32,
        y: f32,
        action: String,
        params: String,
    },
    /// `warp LEVEL X Y`.
    Warp { level: String, x: f32, y: f32 },
    /// `enter`: send the enter-level messages again.
    Enter,
}

/// Reads a steps file. Blank lines and lines starting with `#` are skipped.
/// An error names the line number.
pub fn parse_steps(text: &str) -> Result<Vec<Step>, String> {
    let mut out = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let bad = |why: &str| format!("line {}: {why}: {line}", i + 1);
        let (cmd, rest) = line.split_once(' ').unwrap_or((line, ""));
        let rest = rest.trim();
        let num = |s: Option<&str>| -> Result<f32, String> {
            s.and_then(|v| v.parse().ok())
                .ok_or_else(|| bad("expected a number"))
        };
        let step = match cmd {
            "wait" => Step::Wait(num(Some(rest))?),
            "walk" => {
                let mut p = rest.split_whitespace();
                let x = num(p.next())?;
                let y = num(p.next())?;
                let dir = match p.next() {
                    Some(d) => num(Some(d))? as u8,
                    None => 2,
                };
                Step::Walk { x, y, dir }
            }
            "chat" => Step::Chat(rest.to_string()),
            "trigger" => {
                let p: Vec<&str> = rest.splitn(4, ' ').collect();
                if p.len() < 3 {
                    return Err(bad("trigger needs X Y ACTION"));
                }
                Step::Trigger {
                    x: num(Some(p[0]))?,
                    y: num(Some(p[1]))?,
                    action: p[2].to_string(),
                    params: p.get(3).copied().unwrap_or("").to_string(),
                }
            }
            "warp" => {
                let p: Vec<&str> = rest.split_whitespace().collect();
                if p.len() != 3 {
                    return Err(bad("warp needs LEVEL X Y"));
                }
                Step::Warp {
                    level: p[0].to_string(),
                    x: num(Some(p[1]))?,
                    y: num(Some(p[2]))?,
                }
            }
            "enter" => Step::Enter,
            _ => return Err(bad("unknown step")),
        };
        out.push(step);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_every_step() {
        let s = parse_steps(
            "# c\n\nwait 3\nwalk 16 46 1\nwalk 1 2\nchat hello there\n\
             trigger 30 40 open a,b c\nwarp npcdemo_room.nw 30 30\nenter\n",
        )
        .unwrap();
        assert_eq!(
            s,
            [
                Step::Wait(3.0),
                Step::Walk {
                    x: 16.0,
                    y: 46.0,
                    dir: 1
                },
                Step::Walk {
                    x: 1.0,
                    y: 2.0,
                    dir: 2
                },
                Step::Chat("hello there".into()),
                Step::Trigger {
                    x: 30.0,
                    y: 40.0,
                    action: "open".into(),
                    params: "a,b c".into()
                },
                Step::Warp {
                    level: "npcdemo_room.nw".into(),
                    x: 30.0,
                    y: 30.0
                },
                Step::Enter,
            ]
        );
    }

    #[test]
    fn bad_lines_name_their_number() {
        assert!(parse_steps("wait x").unwrap_err().starts_with("line 1:"));
        assert!(parse_steps("\nwarp a 1")
            .unwrap_err()
            .starts_with("line 2:"));
        assert!(parse_steps("jump").unwrap_err().contains("unknown step"));
    }

    #[test]
    fn the_npcdemo_steps_parse() {
        let text = include_str!("../../../docs/lab/probe/npcdemo-steps.txt");
        assert!(parse_steps(text).unwrap().len() > 10);
    }
}
